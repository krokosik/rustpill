#![no_std]
#![no_main]

use defmt_brtt::{self as _, DefmtConsumer};
use embassy_executor::Spawner;
use embassy_stm32::{
    Config, bind_interrupts,
    gpio::OutputType,
    peripherals,
    time::Hertz,
    timer::{
        self,
        simple_pwm::{PwmPin, SimplePwm, SimplePwmChannel},
    },
    usb,
};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use panic_probe as _;
use portable_atomic::{AtomicBool, Ordering};
use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    server::{
        Dispatch, Sender, Server, SpawnContext,
        impls::embassy_usb_v0_4::dispatch_impl::{WireRxBuf, WireSpawnImpl, spawn_fn},
    },
};
use protocol::{servo::*, utils::PwmChannel};

use firmware::*;
use static_cell::StaticCell;

static BBQ: StaticCell<Mutex<ThreadModeRawMutex, DefmtConsumer>> = StaticCell::new();

struct Context {
    pwm: SimplePwm<'static, peripherals::TIM4>, // Possibly expand to more timers in the future
    config: ServoConfig,
    consumer: &'static Mutex<ThreadModeRawMutex, DefmtConsumer>,
}

struct SpawnCtx {
    consumer: &'static Mutex<ThreadModeRawMutex, DefmtConsumer>,
}

impl SpawnContext for Context {
    type SpawnCtxt = SpawnCtx;
    fn spawn_ctxt(&mut self) -> Self::SpawnCtxt {
        SpawnCtx {
            consumer: self.consumer,
        }
    }
}

type AppServer = Server<AppTx, AppRx, WireRxBuf, App>;

const SERVO_FREQ: Hertz = Hertz(50);
const SERVO_MIN_US: u32 = 500;
const SERVO_MAX_US: u32 = 2500;

bind_interrupts!(struct Irqs {
    TIM4 => timer::CaptureCompareInterruptHandler<peripherals::TIM4>;
    USB_LP_CAN1_RX0 => usb::InterruptHandler<peripherals::USB>;
});

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
        | ConfigureChannel          | blocking  | configure_channel_handler     |
        | GetServoConfig            | blocking  | get_servo_config_handler      |
        | SetFrequencyEndpoint      | blocking  | set_frequency_handler         |
        | StartDefmtLoggingEndpoint | spawn     | defmt_handler_start_logging   |
        | StopDefmtLoggingEndpoint  | blocking  | defmt_handler_stop_logging    |
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
    let consumer = defmt_brtt::init().unwrap();
    let consumer_ref = BBQ.init(Mutex::new(consumer));

    let mut config = Config::default();
    enable_usb_clock(&mut config);
    let mut p = embassy_stm32::init(config);

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
    let max_duty_cycle = pwm.max_duty_cycle();
    defmt::info!("Max Duty Cycle: {}", max_duty_cycle);
    let servo_min = (max_duty_cycle as u32) * SERVO_FREQ.0 / 1_000 * SERVO_MIN_US / 1_000;
    let servo_max = (max_duty_cycle as u32) * SERVO_FREQ.0 / 1_000 * SERVO_MAX_US / 1_000;

    defmt::info!("Servo min: {}, Servo max: {}", servo_min, servo_max);

    let servo_config = ServoConfig {
        servo_frequency: SERVO_FREQ.0,
        max_duty_cycle,
        channels: [ServoChannelConfig {
            min_angle_duty_cycle: servo_min as u16,
            max_angle_duty_cycle: servo_max as u16,
            ..Default::default()
        }; 4],
    };

    // Prepare the context for the application.
    let context = Context {
        config: servo_config,
        pwm,
        consumer: consumer_ref,
    };

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

static STOP: AtomicBool = AtomicBool::new(false);

#[embassy_executor::task]
async fn defmt_handler_start_logging(
    context: SpawnCtx,
    header: VarHeader,
    _rqst: (),
    sender: Sender<AppTx>,
) {
    defmt::info!("defmt start");
    let mut consumer = context.consumer.lock().await;
    if sender
        .reply::<StartDefmtLoggingEndpoint>(header.seq_no, &())
        .await
        .is_err()
    {
        defmt::error!("Failed to send reply to StartDefmtLoggingEndpoint. Stopping...");
        return;
    }

    let mut seq = 0u8;
    let mut buf = [0u8; 32];
    let buf_len = buf.len();

    while !STOP.load(Ordering::Acquire) {
        // This future is broken, on empty log queue it blocks the entire application.
        // Even timeouts don't work with it, probably our own async version is needed.
        let grant = consumer.wait_for_log().await;
        let n = core::cmp::min(grant.len(), buf_len);

        buf[..n].copy_from_slice(&grant[..n]);

        if sender
            .publish::<DefmtLoggingTopic>(seq.into(), &(n as u8, buf))
            .await
            .is_err()
        {
            defmt::error!("Failed to send defmt log chunk. Stopping...");
            return;
        }

        seq = seq.wrapping_add(1);

        grant.release(n);
    }

    let _ = sender
        .publish::<DefmtLoggingTopic>(seq.into(), &(0, buf))
        .await;

    STOP.store(false, Ordering::Release);
}

fn defmt_handler_stop_logging(context: &mut Context, _header: VarHeader, _rqst: ()) -> bool {
    defmt::info!("defmt stop");
    let was_busy = context.consumer.try_lock().is_err();
    if was_busy {
        STOP.store(true, Ordering::Release);
    }
    was_busy
}

#[embassy_executor::task]
async fn server_task(mut server: AppServer) {
    loop {
        // If the host disconnects, we'll return an error here.
        // If this happens, just wait until the host reconnects
        let _ = server.run().await;
    }
}

fn unique_id_handler(_context: &mut Context, _header: VarHeader, _rqst: ()) -> [u8; 24] {
    defmt::info!("unique_id");
    *embassy_stm32::uid::uid_hex_bytes()
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
