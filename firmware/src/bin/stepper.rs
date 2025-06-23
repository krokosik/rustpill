#![no_std]
#![no_main]

//START IMPORTS
use embassy_executor::Spawner;
use embassy_stm32::{
    Config, bind_interrupts,
    gpio::{Level, Output, OutputType, Speed},
    peripherals,
    time::Hertz,
    timer::{
        self,
        simple_pwm::{PwmPin, SimplePwm},
    },
    usb,
};
use embassy_time::{Ticker, Timer};
use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    server::{
        Dispatch, Server,
        impls::embassy_usb_v0_4::dispatch_impl::{WireRxBuf, WireSpawnImpl},
    },
};
use protocol::{
    GetUniqueIdEndpoint, STEPPER_ENDPOINTS_LIST, SetDirectionEndpoint, SetStepperEndpoint,
    TOPICS_IN_LIST, TOPICS_OUT_LIST,
};

use {defmt_rtt as _, panic_probe as _};

use firmware::*;
//END IMPORTS

//context for server
struct Context {
    pwm: SimplePwm<'static, peripherals::TIM4>, // Using TIM1 for PWM
    dir: Output<'static>,
    dma: peripherals::DMA1_CH1, // Specify the GPIO pin type explicitly
}

type AppServer = Server<AppTx, AppRx, WireRxBuf, App>;

define_dispatch! {
    app: App;
    spawn_fn: spawn_fn;
    tx_impl: AppTx;
    spawn_impl: WireSpawnImpl;
    context: Context;

//define endpoint functions, endpoints from protocol/src/lib
    endpoints: {
        list: STEPPER_ENDPOINTS_LIST;

        | EndpointTy                | kind      | handler                       |
        | ----------                | ----      | -------                       |
        | GetUniqueIdEndpoint       | blocking  | unique_id_handler             |
        | SetStepperEndpoint        | async     | set_stepper_handler           |
        | SetDirectionEndpoint      | blocking  | set_direction_handler         |
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

const STEPPER_FREQ: Hertz = Hertz(500);

//binding interrupt functions
bind_interrupts!(struct Irqs {
    TIM4 => timer::CaptureCompareInterruptHandler<peripherals::TIM4>;
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

    let pwm = SimplePwm::new(
        p.TIM4,
        Some(PwmPin::new_ch1(p.PB6, OutputType::PushPull)),
        None,
        None,
        None,
        STEPPER_FREQ,
        timer::low_level::CountingMode::CenterAlignedBothInterrupts,
    );

    let dir = Output::new(p.PB12, Level::Low, Speed::Low);

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

    let dma1 = p.DMA1_CH1;

    let context = Context {
        pwm,
        dir,
        dma: dma1,
    };

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

async fn set_stepper_handler(context: &mut Context, _header: VarHeader, rqst: u32) -> () {
    defmt::info!("set_stepper: {}", rqst);
    let max_duty = context.pwm.max_duty_cycle() / 2;
    context.pwm.ch1().set_duty_cycle(max_duty / 2);
    context.pwm.ch1().enable();

    let mut ticker = Ticker::every(embassy_time::Duration::from_hz(500));
    for _ in 0..rqst {
        context
            .pwm
            .waveform_ch1(&mut context.dma, &[max_duty])
            .await;

        // Timer::after_micros(50).await;

        ticker.next().await;
    }
    context.pwm.ch1().disable();
    // Here you would implement the logic to set the stepper configuration
}

fn set_direction_handler(context: &mut Context, _header: VarHeader, rqst: u8) -> () {
    // Assuming rqst is a direction value, e.g., 0 for one direction and 1 for the other
    if rqst == 0 {
        context.dir.set_low();
    } else if rqst == 1 {
        context.dir.set_high();
    } else {
        defmt::warn!("Invalid direction value: {}", rqst);
        return;
    }
    defmt::info!("set_direction: {}", rqst);
    // Here you would implement the logic to set the stepper direction
}

//END FUNCTIONS FOR ENDPOINTS
