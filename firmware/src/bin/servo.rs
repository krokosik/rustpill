#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::{
    Config, bind_interrupts,
    gpio::{Level, Output, OutputType, Speed},
    peripherals,
    time::Hertz,
    timer::{
        self,
        simple_pwm::{PwmPin, SimplePwm, SimplePwmChannel},
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
use protocol::{
    ConfigureChannel, GetServoConfig, GetUniqueIdEndpoint, PingX2Endpoint, PwmChannel,
    SERVO_ENDPOINT_LIST, ServoChannelConfig, ServoConfig, SetFrequencyEndpoint, TOPICS_IN_LIST,
    TOPICS_OUT_LIST,
};
use {defmt_rtt as _, panic_probe as _};

use firmware::*;

struct Context {
    pwm: SimplePwm<'static, peripherals::TIM4>, // Possibly expand to more timers in the future
    config: ServoConfig,
}

type AppServer = Server<AppTx, AppRx, WireRxBuf, ServoApp>;

const SERVO_FREQ: Hertz = Hertz(50);
const SERVO_MIN_US: u32 = 500;
const SERVO_MAX_US: u32 = 2500;

bind_interrupts!(struct Irqs {
    TIM4 => timer::CaptureCompareInterruptHandler<peripherals::TIM4>;
    USB_LP_CAN1_RX0 => usb::InterruptHandler<peripherals::USB>;
});

define_dispatch! {
    app: ServoApp;
    spawn_fn: spawn_fn;
    tx_impl: AppTx;
    spawn_impl: WireSpawnImpl;
    context: Context;

    endpoints: {
        list: SERVO_ENDPOINT_LIST;

        | EndpointTy                | kind      | handler                       |
        | ----------                | ----      | -------                       |
        | PingX2Endpoint            | blocking  | pingx2_handler                |
        | GetUniqueIdEndpoint       | blocking  | unique_id_handler             |
        | ConfigureChannel          | blocking  | configure_channel_handler     |
        | GetServoConfig            | blocking  | get_servo_config_handler      |
        | SetFrequencyEndpoint      | blocking  | set_frequency_handler         |
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

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    enable_usb_clock(&mut config);
    let mut p = embassy_stm32::init(config);

    let pbufs = PBUFS.take();

    /********************************** PWM **********************************/
    let pwm = SimplePwm::new(
        p.TIM4,
        Some(PwmPin::new_ch1(p.PB6, OutputType::PushPull)),
        Some(PwmPin::new_ch2(p.PB7, OutputType::PushPull)),
        Some(PwmPin::new_ch3(p.PB8, OutputType::PushPull)),
        Some(PwmPin::new_ch4(p.PB9, OutputType::PushPull)),
        SERVO_FREQ,
        timer::low_level::CountingMode::CenterAlignedBothInterrupts,
    );
    let max_duty_cycle = pwm.max_duty_cycle() as u32;
    defmt::info!("Max Duty Cycle: {}", max_duty_cycle);
    let servo_min = max_duty_cycle * SERVO_FREQ.0 / 1_000 * SERVO_MIN_US / 1_000;
    let servo_max = max_duty_cycle * SERVO_FREQ.0 / 1_000 * SERVO_MAX_US / 1_000;

    defmt::info!("Servo min: {}, Servo max: {}", servo_min, servo_max);

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

    let mut servo_config = ServoConfig::default();
    servo_config.servo_frequency = SERVO_FREQ.0;
    servo_config.max_duty_cycle = max_duty_cycle as u16;
    for i in 0..4 {
        servo_config.channels[i].min_angle_duty_cycle = servo_min as u16;
        servo_config.channels[i].max_angle_duty_cycle = servo_max as u16;
        servo_config.channels[i].enabled = false;
    }

    let context = Context {
        config: servo_config,
        pwm,
    };
    let (device, tx_impl, rx_impl) = STORAGE.init(driver, usb_config, pbufs.tx_buf.as_mut_slice());
    let dispatcher = ServoApp::new(context, spawner.into());
    let vkk = dispatcher.min_key_len();
    let server: AppServer = Server::new(
        tx_impl,
        rx_impl,
        pbufs.rx_buf.as_mut_slice(),
        dispatcher,
        vkk,
    );

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

        // This is a workaround for the USB stack to work properly.
        Timer::after_millis(1).await;
    }
}

fn pingx2_handler(_context: &mut Context, _header: VarHeader, rqst: u32) -> u32 {
    defmt::info!("pingx2");
    rqst * 2
}

fn unique_id_handler(_context: &mut Context, _header: VarHeader, _rqst: ()) -> [u8; 12] {
    defmt::info!("unique_id");
    *embassy_stm32::uid::uid()
}

fn configure_channel_handler(
    context: &mut Context,
    _header: VarHeader,
    rqst: (PwmChannel, ServoChannelConfig),
) {
    defmt::info!("configure_channel");

    let (channel, config) = rqst;
    let mut ch = get_channel(&mut context.pwm, channel);

    defmt::info!(
        "Configuring channel {}: {}/{}",
        channel as usize,
        config.current_duty_cycle,
        context.config.max_duty_cycle
    );

    ch.set_duty_cycle(config.current_duty_cycle);
    if config.enabled {
        ch.enable();
    } else {
        ch.disable();
    }
    context.config.channels[channel as usize] = config;
}

fn get_servo_config_handler(context: &mut Context, _header: VarHeader, _rqst: ()) -> ServoConfig {
    defmt::info!("get_servo_config");
    context.config.clone()
}

fn get_channel<'d>(
    pwm: &'d mut SimplePwm<peripherals::TIM4>,
    channel: PwmChannel,
) -> SimplePwmChannel<'d, peripherals::TIM4> {
    match channel {
        PwmChannel::Channel1 => pwm.ch1(),
        PwmChannel::Channel2 => pwm.ch2(),
        PwmChannel::Channel3 => pwm.ch3(),
        PwmChannel::Channel4 => pwm.ch4(),
    }
}

fn set_frequency_handler(context: &mut Context, _header: VarHeader, rqst: u32) {
    defmt::info!("set_frequency");

    context.pwm.ch1().disable();
    context.pwm.ch2().disable();
    context.pwm.ch3().disable();
    context.pwm.ch4().disable();

    context.pwm.set_frequency(Hertz(rqst));
    defmt::warn!(
        "Frequency change, max duty cycle changed from {} to {}. Disabling all channels...",
        context.config.max_duty_cycle,
        context.pwm.max_duty_cycle()
    );

    for i in 0..4 {
        context.config.channels[i].enabled = false;
    }
}
