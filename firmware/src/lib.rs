#![no_std]
#![no_main]

use embassy_stm32::{
    Config, Peripheral,
    gpio::{Level, Output, Speed},
    peripherals,
    time::Hertz,
    usb::{self, DpPin, Instance},
};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_time::Timer;
use embassy_usb::UsbDevice;
use postcard_rpc::server::impls::embassy_usb_v0_4::{
    PacketBuffers,
    dispatch_impl::{WireRxImpl, WireStorage, WireTxImpl},
};
use static_cell::ConstStaticCell;

pub type AppDriver = usb::Driver<'static, peripherals::USB>;
pub type AppStorage = WireStorage<ThreadModeRawMutex, AppDriver, 256, 256, 64, 256>;
pub type BufStorage = PacketBuffers<1024, 1024>;
pub type AppTx = WireTxImpl<ThreadModeRawMutex, AppDriver>;
pub type AppRx = WireRxImpl<AppDriver>;

pub static PBUFS: ConstStaticCell<BufStorage> = ConstStaticCell::new(BufStorage::new());
pub static STORAGE: AppStorage = AppStorage::new();

pub fn enable_usb_clock(config: &mut Config) {
    use embassy_stm32::rcc::*;
    config.rcc.hse = Some(Hse {
        freq: Hertz(8_000_000),
        // Oscillator for bluepill, Bypass for nucleos.
        mode: HseMode::Oscillator,
    });
    config.rcc.pll = Some(Pll {
        src: PllSource::HSE,
        prediv: PllPreDiv::DIV1,
        mul: PllMul::MUL9,
    });
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    config.rcc.apb1_pre = APBPrescaler::DIV2;
    config.rcc.apb2_pre = APBPrescaler::DIV1;
}

pub async fn reset_condition<T: Instance>(dplus_pin: &mut impl Peripheral<P = impl DpPin<T>>) {
    // BluePill board has a pull-up resistor on the D+ line.
    // Pull the D+ pin down to send a RESET condition to the USB bus.
    // This forced reset is needed only for development, without it host
    // will not reset your device when you upload new firmware.
    let _dp = Output::new(dplus_pin, Level::Low, Speed::Low);
    Timer::after_millis(10).await;
}

pub fn get_usb_config(product_name: &'static str) -> embassy_usb::Config<'static> {
    let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("QOD Lab");
    config.product = Some(product_name);
    config.serial_number = Some(embassy_stm32::uid::uid_hex());

    defmt::info!("Serial number: {}", embassy_stm32::uid::uid_hex());

    let version_bcd = 0
        + (env!("CARGO_PKG_VERSION_MAJOR").parse::<u16>().unwrap() << 8)
        + (env!("CARGO_PKG_VERSION_MINOR").parse::<u16>().unwrap() << 4)
        + env!("CARGO_PKG_VERSION_PATCH").parse::<u16>().unwrap();

    config.device_release = version_bcd;

    // Required for windows compatibility.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    config
}

#[embassy_executor::task]
pub async fn idle_task() {
    loop {
        embassy_futures::yield_now().await;
    }
}

/// This handles the low level USB management
#[embassy_executor::task]
pub async fn usb_task(mut usb: UsbDevice<'static, AppDriver>) {
    defmt::info!("USB started");
    usb.run().await;
}
