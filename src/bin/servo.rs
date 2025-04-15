#![no_std]
#![no_main]

use defmt::{panic, *};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_stm32::gpio::{Level, Output, OutputType, Speed};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::usb::Driver;
use embassy_stm32::{bind_interrupts, peripherals, timer, usb, Config};
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::Builder;
use {defmt_rtt as _, panic_probe as _};

use rustpill::enable_usb_clock;

const SERVO_FREQ: Hertz = Hertz(50);
const SERVO_MIN_US: u32 = 500;
const SERVO_MAX_US: u32 = 2500;

bind_interrupts!(struct Irqs {
    TIM4 => timer::CaptureCompareInterruptHandler<peripherals::TIM4>;
    USB_LP_CAN1_RX0 => usb::InterruptHandler<peripherals::USB>;
});

#[embassy_executor::task]
async fn idle() {
    loop {
        embassy_futures::yield_now().await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    enable_usb_clock(&mut config);
    let mut p = embassy_stm32::init(config);

    spawner.spawn(idle()).unwrap();

    /********************************** PWM **********************************/
    let mut pwm = SimplePwm::new(
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

    let mut pwm = pwm.ch2();
    pwm.enable();

    let angle_to_duty_cycle = |angle: u8| {
        let duty_cycle = servo_min + angle as u32 * (servo_max - servo_min) / 180;
        if duty_cycle < servo_min {
            servo_min as u16
        } else if duty_cycle > servo_max {
            servo_max as u16
        } else {
            duty_cycle as u16
        }
    };

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
    let driver = Driver::new(p.USB, Irqs, p.PA12, p.PA11);

    // Create embassy-usb Config
    let config = embassy_usb::Config::new(0xc0de, 0xcafe);

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 7];

    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [], // no msos descriptors
        &mut control_buf,
    );

    // Create classes on the builder.
    let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    /********************************** LOOP **********************************/
    let echo_fut = async {
        loop {
            class.wait_connection().await;
            info!("Connected");
            let mut buf = [0; 64];
            loop {
                match class.read_packet(&mut buf).await {
                    Ok(n) => {
                        let data = &buf[..n];
                        info!("data: {:x}", data);

                        let angle = data[0];

                        let duty_cycle = angle_to_duty_cycle(angle);
                        info!("angle: {}, duty_cycle: {}", angle, duty_cycle);

                        pwm.set_duty_cycle(duty_cycle);
                    }
                    Err(_) => break,
                }
            }
            info!("Disconnected");
        }
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, echo_fut).await;
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}
