#![no_std]
#![no_main]

//START IMPORTS
use embassy_executor::Spawner;
use embassy_futures::block_on;
use embassy_stm32::{
    Config,
    adc::Adc,
    bind_interrupts,
    gpio::{Level, Output, OutputType, Speed},
    peripherals::{self, PA0},
    time::Hertz,
    timer::{
        self,
        simple_pwm::{PwmPin, SimplePwm},
    },
    usb,
};
use embassy_time::Timer;
use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    server::{
        Dispatch, Server,
        impls::embassy_usb_v0_4::dispatch_impl::{WireRxBuf, WireSpawnImpl},
    },
};

use protocol::heater::*;

use {defmt_rtt as _, panic_probe as _};

use firmware::*;
//END IMPORTS

/***context for server
 * **/
struct Context {
    adc: Adc<'static, peripherals::ADC1>,
    adc_pin: PA0,
    heater_pwm: SimplePwm<'static, peripherals::TIM2>,
    pidvals: Pidvals,
}

type AppServer = Server<AppTx, AppRx, WireRxBuf, App>;

define_dispatch! {
    app: App;
    spawn_fn: spawn_fn;
    tx_impl: AppTx;
    spawn_impl: WireSpawnImpl;
    context: Context;

    endpoints: {
        list: ENDPOINT_LIST;

        | EndpointTy                | kind      | handler                       |
        | ----------                | ----      | -------                       |
        | GetUniqueIdEndpoint       | blocking  | unique_id_handler             |
        | SetPWMDutyEndpoint        | blocking  | set_pwm_duty                  |
        | GetPidvalsEndpoint        | blocking  | get_pidvals                   |
        | HeaterDisableEndpoint     | blocking  | heater_disable                |
        | HeaterEnableEndpoint      | blocking  | heater_enable                 |
        | SetPidConstsEndpoint      | blocking  | pid_set_const                 |
        | PidResetEndpoint          | blocking  | pid_reset                     |
        | RecalcPIEndpoint          | blocking  | recalc_pi                     |
        | SetPISetpoint             | blocking  | set_setpoint                  |
    };
    topics_in: {
        list: TOPICS_IN_LIST;

        | TopicTy                   | kind      | handler                       |
        | ----------                | ----      | -------                       |
    };
    topics_out: {
        list: TOPICS_OUT_LIST;
    };
}

//binding interrupt functions
bind_interrupts!(struct Irqs {
    USB_LP_CAN1_RX0 => usb::InterruptHandler<peripherals::USB>;
});

//first function of programme
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    //config with clocks
    let mut config = Config::default();
    enable_usb_clock(&mut config);

    //START INIT PERIPHERALS
    let mut p = embassy_stm32::init(config);

    let pbufs = PBUFS.take();

    /********************************** START ADC, PWM **********************************/
    let pidvals = Pidvals {
        setpoint: 0,
        adc_val: 0,
        error_val: 0,
        err_corr: 0.,
        kp: 0.,
        ki: 0.,
        dt: 1,
        prev_clk: embassy_time::Instant::now().as_ticks(),
        is_on: false,
        max_int_val: 1000.,
        int_sum: 0.,
    };
    let context = Context {
        adc: Adc::new(p.ADC1),
        adc_pin: p.PA0,
        heater_pwm: SimplePwm::new(
            p.TIM2,
            None,
            Some(PwmPin::new_ch2(p.PA1, OutputType::PushPull)),
            None,
            None,
            Hertz(60),
            timer::low_level::CountingMode::EdgeAlignedUp,
        ),
        pidvals: pidvals,
    };
    // context.heater_pwm.ch2().enable(); // for testing
    // context.heater_pwm.ch2().set_duty_cycle_percent(50);
    /********************************** END ADC, PWM **********************************/
    /********************************** USB **********************************/
    {
        // BluePill board has a pull-up resistor on the D+ line.
        // Pull the D+ pin down to send a RESET condition to the USB bus.
        // This forced reset is needed only for development, without it host
        // will not reset your device when you upload new firmware.

        let _dp = Output::new(&mut p.PA12, Level::Low, Speed::Low);
        Timer::after_millis(10).await;
    }

    // Create the driver, from the HAL.
    let driver = usb::Driver::new(p.USB, Irqs, p.PA12, p.PA11);

    // Create embassy-usb Config
    let usb_config = get_usb_config("bluepill-servo");

    let (device, tx_impl, rx_impl) = STORAGE.init(driver, usb_config, pbufs.tx_buf.as_mut_slice());
    let dispatcher = App::new(context, spawner.into());
    let vkk = dispatcher.min_key_len();
    let server: AppServer = Server::new(
        tx_impl,
        rx_impl,
        pbufs.rx_buf.as_mut_slice(),
        dispatcher,
        vkk,
    );
    /********************************** END USB **********************************/

    //END INIT PERIPHERALS

    //spawn tasks
    spawner.must_spawn(usb_task(device));
    spawner.must_spawn(server_task(server));
    spawner.must_spawn(idle_task());
}

#[embassy_executor::task]
async fn server_task(mut server: AppServer) {
    loop {
        // If the host disconnects, we'll return an error here.
        // If this happens, just wait until the host reconnects
        let _ = server.run().await;
    }
}

//START FUNCTIONS FOR ENDPOINTS

fn unique_id_handler(_context: &mut Context, _header: VarHeader, _rqst: ()) -> [u8; 12] {
    defmt::info!("unique_id");
    *embassy_stm32::uid::uid()
}

fn set_pwm_duty(context: &mut Context, _header: VarHeader, rqst: u16) -> () {
    //todo arg duty
    context.heater_pwm.ch2().set_duty_cycle_fraction(rqst, 1000);
}
fn pid_reset(context: &mut Context, _header: VarHeader, _rqst: ()) -> () {
    context.pidvals.prev_clk = embassy_time::Instant::now().as_ticks();
    context.pidvals.int_sum = 0.;
}
fn pid_set_const(context: &mut Context, _header: VarHeader, rqst: [f32; 2]) -> () {
    context.pidvals.kp = rqst[0];
    context.pidvals.ki = rqst[1];
}

fn heater_enable(context: &mut Context, _header: VarHeader, _rqst: ()) -> () {
    context.heater_pwm.ch2().enable();
}
fn heater_disable(context: &mut Context, _header: VarHeader, _rqst: ()) -> () {
    context.heater_pwm.ch2().disable();
}

fn recalc_pi(context: &mut Context, _header: VarHeader, _rqst: ()) -> () {
    context.pidvals.adc_val = block_on(context.adc.read(&mut context.adc_pin));
    let now = embassy_time::Instant::now().as_ticks();
    context.pidvals.dt = now.wrapping_sub(context.pidvals.prev_clk);
    context.pidvals.prev_clk = now;

    context.pidvals.error_val =
        (context.pidvals.adc_val as i16) - (context.pidvals.setpoint as i16);

    context.pidvals.int_sum += context.pidvals.ki * context.pidvals.error_val as f32;
    if context.pidvals.int_sum > context.pidvals.max_int_val {
        context.pidvals.int_sum = context.pidvals.max_int_val;
    } else if context.pidvals.int_sum < -context.pidvals.max_int_val {
        context.pidvals.int_sum = -context.pidvals.max_int_val;
    }

    context.pidvals.err_corr =
        context.pidvals.kp * (context.pidvals.error_val as f32) + context.pidvals.int_sum;
    if context.pidvals.err_corr < 0. {
        context.heater_pwm.ch2().set_duty_cycle(0);
    } else if context.pidvals.err_corr > 1000. {
        context.heater_pwm.ch2().set_duty_cycle_fully_on();
    } else {
        context
            .heater_pwm
            .ch2()
            .set_duty_cycle_fraction(context.pidvals.err_corr as u16, 1000);
    }
}

fn get_pidvals(context: &mut Context, _header: VarHeader, _rqst: ()) -> Pidvals {
    context.pidvals.clone()
}

fn set_setpoint(context: &mut Context, _header: VarHeader, rqst: u16) -> () {
    context.pidvals.setpoint = rqst;
}

//END FUNCTIONS FOR ENDPOINTS
