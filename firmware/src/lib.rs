#![no_std]
#![no_main]

use embassy_stm32::{time::Hertz, Config};

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
