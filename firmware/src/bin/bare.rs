#![no_std]
#![no_main]

use core::iter::Empty;

use embassy_executor::Spawner;
use embassy_stm32::{
    Config, bind_interrupts,
    gpio::{Level, Output, Speed},
    peripherals, usb,
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
use protocol::{ENDPOINT_LIST, EmptyConfig, GetUniqueIdEndpoint, TOPICS_IN_LIST, TOPICS_OUT_LIST};

use {defmt_rtt as _, panic_probe as _};

use firmware::*;

struct Context {
    // pwm: SimplePwm<'static, peripherals::TIM4>, // Possibly expand to more timers in the future
    config: EmptyConfig,
}

type AppServer = Server<AppTx, AppRx, WireRxBuf, AdcApp>;

// const SERVO_FREQ: Hertz = Hertz(50);
// const SERVO_MIN_US: u32 = 500;
// const SERVO_MAX_US: u32 = 2500;

bind_interrupts!(struct Irqs {
    // TIM4 => timer::CaptureCompareInterruptHandler<peripherals::TIM4>;
    USB_LP_CAN1_RX0 => usb::InterruptHandler<peripherals::USB>;
});

define_dispatch! {
    app: AdcApp;
    spawn_fn: spawn_fn;
    tx_impl: AppTx;
    spawn_impl: WireSpawnImpl;
    context: Context;

    endpoints: {
        list: ENDPOINT_LIST;

        | EndpointTy                | kind      | handler                       |
        | ----------                | ----      | -------                       |
        | GetUniqueIdEndpoint       | blocking  | unique_id_handler             |
        // | ConfigureChannel          | blocking  | configure_channel_handler     |
        // | GetServoConfig            | blocking  | get_servo_config_handler      |
        // | SetFrequencyEndpoint      | blocking  | set_frequency_handler         |
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

    let mut context_config = EmptyConfig::default();

    let context = Context {
        config: context_config,
    };
    let (device, tx_impl, rx_impl) = STORAGE.init(driver, usb_config, pbufs.tx_buf.as_mut_slice());
    let dispatcher = AdcApp::new(context, spawner.into());
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
    }
}

fn unique_id_handler(_context: &mut Context, _header: VarHeader, _rqst: ()) -> [u8; 12] {
    defmt::info!("unique_id");
    *embassy_stm32::uid::uid()
}
