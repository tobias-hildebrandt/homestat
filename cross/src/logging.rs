use core::fmt::{Display, Formatter};

use chrono::TimeDelta;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::Peri;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver as UsbDriver;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcAcmState};
use embassy_usb::{Builder as UsbBuilder, Config as UsbConfig};
use embassy_usb_logger::ReceiverHandler;

use crate::Irqs;

const MAX_PACKET_SIZE: u8 = 64;
const LOG_BUFFER_SIZE: usize = 1024;
const LOG_FILTER: log::LevelFilter = log::LevelFilter::Info;

/// Sets up USB logging, spawns logger task.
pub fn setup_usb_logging(spawner: Spawner, usb: Peri<'static, USB>) {
    let usb_driver = UsbDriver::new(usb, Irqs);
    spawner.spawn(logger_task(usb_driver)).unwrap();
}

/// USB logging task.
#[embassy_executor::task]
async fn logger_task(driver: UsbDriver<'static, USB>) -> ! {
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
            let module = record.module_path().unwrap_or("(no module)");
            write!(
                writer,
                "[{time}] [{level:5}] [{module}] {}\r\n",
                record.args()
            )
            .unwrap();
        },
        embassy_usb_logger::DummyHandler
    );

    let mut usb = builder.build();
    let usb_fut = usb.run();
    join(usb_fut, log_fut).await;

    panic!("usb logger task ended");
}

/// Formatted timestamp based on clock ticks.
pub struct TimeLog(u64);

impl TimeLog {
    /// Gets the current timestamp.
    pub fn now() -> Self {
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
