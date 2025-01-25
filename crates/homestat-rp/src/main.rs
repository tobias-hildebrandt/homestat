#![no_std]
#![no_main]

mod init;
mod logging;

use init::{InitPins, init_fw_and_pins};
use logging::setup_usb_logging;

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::pac::rtc::regs::{Rtc0, Rtc1};
use embassy_rp::peripherals::{PIO0, USB};
use embassy_rp::pio::InterruptHandler as PioInterruptHandler;
use embassy_rp::rtc::Rtc;
use embassy_rp::usb::InterruptHandler as UsbInterruptHandler;
use embassy_time::Timer;
use log::info;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

// need to import to top-level
use {defmt_rtt as _, panic_probe as _};

// set up interrupts
bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

/// Entrypoint
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let periphs = embassy_rp::init(Default::default());

    let init_pins = InitPins {
        pin23: periphs.PIN_23,
        pin24: periphs.PIN_24,
        pin25: periphs.PIN_25,
        pin29: periphs.PIN_29,
        pio0: periphs.PIO0,
        dma_ch: periphs.DMA_CH0,
    };

    // immediately set up USB logging
    setup_usb_logging(&spawner, periphs.USB);

    // wait til it's OK
    // TODO: replace with channel
    Timer::after_millis(100).await;

    info!("usb logging complete");

    // TODO: network sync rtc, adjust TimeLog format to be relative
    let mut rtc = Rtc::new(periphs.RTC);
    if !rtc.is_running() {
        rtc.restore(Rtc1(0), Rtc0(0));
    }

    // init firmware, network, and pins
    let _control = init_fw_and_pins(&spawner, init_pins).await;

    // jump to main loop
    main_loop().await;
}

async fn main_loop() -> ! {
    // rng and main loop
    let mut rng = SmallRng::seed_from_u64(embassy_time::Instant::now().as_micros());
    let mut count = 0;
    loop {
        info!("count: {count}");
        Timer::after_millis(rng.gen_range(100..1000)).await;

        count += 1;
    }
}
