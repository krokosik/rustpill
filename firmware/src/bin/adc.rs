#![no_std]
#![no_main]

//START IMPORTS
use embassy_executor::Spawner;
use embassy_futures::block_on;
use embassy_stm32::{
    Config,
    adc::Adc,
    bind_interrupts,
    gpio::{Level, Output, Speed},
    peripherals::{self, PA0},
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
    ENDPOINT_LIST, GetAdcValEndpoint, GetUniqueIdEndpoint, TOPICS_IN_LIST, TOPICS_OUT_LIST,
};

use {defmt_rtt as _, panic_probe as _};

use firmware::*;
//END IMPORTS

/***context for server
 * **/
struct Context {
    adc: Adc<'static, peripherals::ADC1>,
    adc_pin: PA0,
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
        | GetAdcValEndpoint         | blocking  | get_adc_val                   |
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

    /********************************** START ADC **********************************/
    let context = Context {
        adc: Adc::new(p.ADC1),
        adc_pin: p.PA0,
    };
    /********************************** END ADC **********************************/
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

fn get_adc_val(context: &mut Context, _header: VarHeader, _rqst: ()) -> u16 {
    block_on(context.adc.read(&mut context.adc_pin))
}
//END FUNCTIONS FOR ENDPOINTS
