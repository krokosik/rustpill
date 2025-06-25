#![no_std]
#![no_main]

/************************** 3rd party imports  **************************/
use embassy_executor::Spawner;
use embassy_stm32::{Config, bind_interrupts, peripherals, usb};
use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    server::{
        Dispatch, Server,
        impls::embassy_usb_v0_4::dispatch_impl::{WireRxBuf, WireSpawnImpl},
    },
};
use {defmt_rtt as _, panic_probe as _};

/**************************** Local imports  ****************************/
use firmware::*;
use protocol::minimal::*; // Change minimal to your protocol module

/*************************** Global objects  ****************************/
// Objects to be shared across handlers.
struct Context {}

// Global type based on the protocol. No need to change this.
type AppServer = Server<AppTx, AppRx, WireRxBuf, App>;

// Bind additional interrupts for used peripherals to use async API.
bind_interrupts!(struct Irqs {
    USB_LP_CAN1_RX0 => usb::InterruptHandler<peripherals::USB>;
});

// Define the dispatch for the application by picking the endpoints/topics you need
// and assigning handlers to them. You can think of this as a router for the incoming requests.
// Endpoints are the Request/Response pairs, while Topics are like Pub/Sub channels.
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

/***************************** MAIN ******************************/
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    enable_usb_clock(&mut config);
    let mut p = embassy_stm32::init(config);

    /******************************** Peri ***********************************/
    // Initialize the peripherals needed for the application and store them in the context if needed.
    // Probably the only block you need to change for your application.

    // Prepare the context for the application.
    let context = Context {};

    /********************************** USB **********************************/
    reset_condition(&mut p.PA12).await;

    // Create the driver, from the HAL.
    let driver = usb::Driver::new(p.USB, Irqs, p.PA12, p.PA11);

    // Create embassy-usb Config
    let usb_config = get_usb_config(USB_DEVICE_NAME);

    let pbufs = PBUFS.take();
    let (device, tx_impl, rx_impl) = STORAGE.init(driver, usb_config, pbufs.tx_buf.as_mut_slice());
    let dispatcher = App::new(context, spawner.into());
    let vkk = dispatcher.min_key_len();
    let server = AppServer::new(
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

/***************************** HANDLERS ******************************/
fn unique_id_handler(_context: &mut Context, _header: VarHeader, _rqst: ()) -> [u8; 24] {
    defmt::info!("unique_id");
    *embassy_stm32::uid::uid_hex_bytes()
}
