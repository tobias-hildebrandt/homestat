#![no_std]
#![no_main]

use core::fmt::{Display, Formatter};

use chrono::TimeDelta;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::pac::rtc::regs::{Rtc0, Rtc1};
use embassy_rp::peripherals::{DMA_CH0, PIO0, USB};
use embassy_rp::pio::{InterruptHandler as PioInterruptHandler, Pio};
use embassy_rp::rtc::Rtc;
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcAcmState};
use embassy_usb::{Builder as UsbBuilder, Config as UsbConfig};
use embassy_usb_logger::ReceiverHandler;
use log::info;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let periphs = embassy_rp::init(Default::default());

    // firmware
    let fw = include_bytes!("../../../misc/43439A0.bin");
    // wifi firmware?
    let clm = include_bytes!("../../../misc/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let usb_driver = UsbDriver::new(periphs.USB, Irqs);
    spawner.spawn(logger_task(usb_driver)).unwrap();

    Timer::after_millis(500).await;

    // TODO: network sync rtc, adjust TimeLog format to be relative
    let mut rtc = Rtc::new(periphs.RTC);
    if !rtc.is_running() {
        rtc.restore(Rtc1(0), Rtc0(0));
    }

    // configure pins
    let pwr = Output::new(periphs.PIN_23, Level::Low);
    let cs = Output::new(periphs.PIN_25, Level::High);
    let mut pio = Pio::new(periphs.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        periphs.PIN_24,
        periphs.PIN_29,
        periphs.DMA_CH0,
    );

    // set up pico w cym43 (network, pins, etc)
    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner.spawn(cyw43_task(runner)).unwrap();

    // control
    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;
    control.gpio_set(0, true).await;

    // rng and main loop
    let mut rng = SmallRng::seed_from_u64(embassy_time::Instant::now().as_micros());
    let mut count = 0;
    loop {
        info!("count: {count}");
        Timer::after_millis(rng.gen_range(100..1000)).await;

        count += 1;
    }
}

#[embassy_executor::task]
async fn logger_task(driver: UsbDriver<'static, USB>) -> ! {
    const MAX_PACKET_SIZE: u8 = 64;
    const LOG_BUFFER_SIZE: usize = 1024;
    const LOG_FILTER: log::LevelFilter = log::LevelFilter::Info;

    let mut config = UsbConfig::new(0xaaaa, 0xaaaa);
    config.manufacturer = Some("todo");
    config.product = Some("todo");
    config.serial_number = Some("1");
    config.max_power = 100;
    config.max_packet_size_0 = MAX_PACKET_SIZE;

    // buffers
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let mut logger_state = CdcAcmState::new();

    let mut builder = UsbBuilder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [],
        &mut control_buf,
    );

    let usb_serial_class =
        CdcAcmClass::new(&mut builder, &mut logger_state, MAX_PACKET_SIZE.into());

    let log_fut = embassy_usb_logger::with_custom_style!(
        LOG_BUFFER_SIZE,
        LOG_FILTER,
        usb_serial_class,
        |record, writer| {
            use core::fmt::Write;
            let level = record.level().as_str();
            let time = TimeLog::now();
            write!(writer, "[{time}] [{level}] {}\r\n", record.args()).unwrap();
        },
        embassy_usb_logger::DummyHandler
    );

    let mut usb = builder.build();
    let usb_fut = usb.run();
    join(usb_fut, log_fut).await;

    panic!("usb logger task ended");
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

struct TimeLog(u64);

impl TimeLog {
    fn now() -> Self {
        Self(embassy_time::Instant::now().as_micros())
    }
}

impl Display for TimeLog {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let time = TimeDelta::microseconds(self.0.try_into().unwrap());

        let days_total = time.num_days();
        let hours = time.num_hours();
        let mins = time.num_minutes();
        let seconds = time.num_seconds();
        let micros = time
            .num_microseconds()
            .unwrap_or(-1)
            .checked_sub(seconds * 1_000_000)
            .unwrap_or(-1);

        write!(f, "{:03}d ", days_total)?;
        write!(f, "{:02}:", hours)?;
        write!(f, "{:02}:", mins)?;
        write!(f, "{:02}.", seconds)?;
        write!(f, "{:06}", micros)
    }
}
