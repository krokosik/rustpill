#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, OutputType, Speed};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::usb;
use embassy_stm32::{Config, bind_interrupts, peripherals, timer};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_time::Timer;
use embassy_usb::UsbDevice;
use postcard_rpc::define_dispatch;
use postcard_rpc::header::VarHeader;
use postcard_rpc::server::impls::embassy_usb_v0_4::PacketBuffers;
use postcard_rpc::server::impls::embassy_usb_v0_4::dispatch_impl::{
    WireRxBuf, WireRxImpl, WireSpawnImpl, WireStorage, WireTxImpl,
};
use postcard_rpc::server::{Dispatch, Server};
use protocol::{
    ENDPOINT_LIST, GetAngleEndpoint, SetAngle, SetAngleEndpoint, TOPICS_IN_LIST, TOPICS_OUT_LIST,
};
use static_cell::ConstStaticCell;
use {defmt_rtt as _, panic_probe as _};

use firmware::enable_usb_clock;

pub struct Context {
    pwm: SimplePwm<'static, peripherals::TIM4>,
    servo_min: u16,
    servo_max: u16,
}

type AppDriver = usb::Driver<'static, peripherals::USB>;
type AppStorage = WireStorage<ThreadModeRawMutex, AppDriver, 256, 256, 64, 256>;
type BufStorage = PacketBuffers<1024, 1024>;
type AppTx = WireTxImpl<ThreadModeRawMutex, AppDriver>;
type AppRx = WireRxImpl<AppDriver>;
type AppServer = Server<AppTx, AppRx, WireRxBuf, MyApp>;

static PBUFS: ConstStaticCell<BufStorage> = ConstStaticCell::new(BufStorage::new());
static STORAGE: AppStorage = AppStorage::new();

const SERVO_FREQ: Hertz = Hertz(50);
const SERVO_MIN_US: u32 = 500;
const SERVO_MAX_US: u32 = 2500;

bind_interrupts!(struct Irqs {
    TIM4 => timer::CaptureCompareInterruptHandler<peripherals::TIM4>;
    USB_LP_CAN1_RX0 => usb::InterruptHandler<peripherals::USB>;
});

define_dispatch! {
    app: MyApp;
    spawn_fn: spawn_fn;
    tx_impl: AppTx;
    spawn_impl: WireSpawnImpl;
    context: Context;

    endpoints: {
        list: ENDPOINT_LIST;

        | EndpointTy                | kind      | handler                       |
        | ----------                | ----      | -------                       |
        | SetAngleEndpoint          | blocking  | set_angle_handler             |
        | GetAngleEndpoint          | blocking  | get_angle_handler             |
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

fn usb_config() -> embassy_usb::Config<'static> {
    let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("QOD Lab");
    config.product = Some("bluepill-servo");
    config.serial_number = Some("2137");

    // Required for windows compatibility.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    config
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    enable_usb_clock(&mut config);
    let mut p = embassy_stm32::init(config);

    spawner.must_spawn(idle());

    let pbufs = PBUFS.take();

    /********************************** PWM **********************************/
    let pwm = SimplePwm::new(
        p.TIM4,
        None,
        Some(PwmPin::new_ch2(p.PB7, OutputType::PushPull)),
        None,
        None,
        SERVO_FREQ,
        timer::low_level::CountingMode::CenterAlignedBothInterrupts,
    );
    let max_duty_cycle = pwm.max_duty_cycle() as u32;
    info!("Max Duty Cycle: {}", max_duty_cycle);
    let servo_min = max_duty_cycle * SERVO_FREQ.0 / 1_000 * SERVO_MIN_US / 1_000;
    let servo_max = max_duty_cycle * SERVO_FREQ.0 / 1_000 * SERVO_MAX_US / 1_000;

    info!("Servo min: {}, Servo max: {}", servo_min, servo_max);

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
    let config = usb_config();

    let context = Context {
        pwm,
        servo_min: servo_min as u16,
        servo_max: servo_max as u16,
    };
    let (device, tx_impl, rx_impl) = STORAGE.init(driver, config, pbufs.tx_buf.as_mut_slice());
    let dispather = MyApp::new(context, spawner.into());
    let vkk = dispather.min_key_len();
    let mut server: AppServer = Server::new(
        tx_impl,
        rx_impl,
        pbufs.rx_buf.as_mut_slice(),
        dispather,
        vkk,
    );

    spawner.must_spawn(usb_task(device));

    loop {
        // If the host disconnects, we'll return an error here.
        // If this happens, just wait until the host reconnects
        let _ = server.run().await;
    }
}

#[embassy_executor::task]
async fn idle() {
    loop {
        embassy_futures::yield_now().await;
    }
}

/// This handles the low level USB management
#[embassy_executor::task]
pub async fn usb_task(mut usb: UsbDevice<'static, AppDriver>) {
    usb.run().await;
}

fn set_angle_handler(context: &mut Context, _header: VarHeader, rqst: SetAngle) {
    let mut duty_cycle = (context.servo_min as u32
        + rqst.angle as u32 * (context.servo_max - context.servo_min) as u32 / 180)
        as u16;
    if duty_cycle < context.servo_min {
        duty_cycle = context.servo_min;
    } else if duty_cycle > context.servo_max {
        duty_cycle = context.servo_max;
    }

    context.pwm.ch2().set_duty_cycle(duty_cycle);
}

fn get_angle_handler(context: &mut Context, _header: VarHeader, _rqst: ()) -> u8 {
    let duty_cycle = context.pwm.ch2().current_duty_cycle();

    ((duty_cycle - context.servo_min) * 180 / (context.servo_max - context.servo_min)) as u8
}
