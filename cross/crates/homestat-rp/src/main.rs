#![no_std]
#![no_main]

mod dht11;
mod init;
mod logging;
mod net;
mod rng;
mod sleep;

use core::time::Duration;

use dht11::spawn_dht11;
use embassy_rp::gpio::Output;
use embassy_time::Timer;
use init::{InitPins, init_fw_and_pins};
use logging::setup_usb_logging;

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::pac::rtc::regs::{Rtc0, Rtc1};
use embassy_rp::peripherals::{PIO0, USB};
use embassy_rp::pio::InterruptHandler as PioInterruptHandler;
use embassy_rp::rtc::Rtc;
use embassy_rp::usb::InterruptHandler as UsbInterruptHandler;
use log::info;
use net::spawn_net;

// need to import to top-level
use {defmt_rtt as _, panic_probe as _};

// set up interrupts
bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
    RTC_IRQ => RtcInterruptHandler;
});

/// Entrypoint
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let periphs = embassy_rp::init(Default::default());
    let cortex_periphs = cortex_m::Peripherals::take().unwrap();

    let mut scb: cortex_m::peripheral::SCB = cortex_periphs.SCB;

    let init_pins = InitPins {
        pin23: periphs.PIN_23,
        pin24: periphs.PIN_24,
        pin25: periphs.PIN_25,
        pin29: periphs.PIN_29,
        pio0: periphs.PIO0,
        dma_ch: periphs.DMA_CH0,
    };

    // TODO: network sync rtc, adjust TimeLog format to be relative
    let mut rtc = Rtc::new(periphs.RTC);
    if !rtc.is_running() {
        rtc.restore(Rtc1(0), Rtc0(0));
    }

    // immediately set up USB logging
    setup_usb_logging(spawner, periphs.USB);

    Timer::after_millis(1000).await;

    info!("usb logging complete");

    // init firmware, network, and pins
    let (stack, control) = init_fw_and_pins(spawner, init_pins).await;

    // spawn dht11 monitor
    spawn_dht11(spawner, periphs.PIN_21);

    // spawn network task
    spawn_net(spawner, stack, control);

    // LED that is set low during sleep
    let mut led = Output::new(periphs.PIN_18, true.into());

    loop {
        Timer::after_secs(30).await;
        const SLEEP_TIME: Duration = Duration::from_secs(5);

        let now = rtc.now().unwrap();

        info!(
            "rtc now is {:?}, sleeping for {}s",
            now,
            SLEEP_TIME.as_secs()
        );
        Timer::after_millis(200).await;

        led.set_low();

        sleep::deep_sleep_xosc(&mut scb, SLEEP_TIME, &mut rtc);

        led.set_high();

        Timer::after_millis(1).await;

        // TODO: restart usb
        info!("done with light sleep, now {:?}", rtc.now().unwrap());
    }
}

/// Interrupt handler.
pub struct RtcInterruptHandler {
    _empty: (),
}

impl embassy_rp::interrupt::typelevel::Handler<embassy_rp::interrupt::typelevel::RTC_IRQ>
    for RtcInterruptHandler
{
    unsafe fn on_interrupt() {
        // disable match
        let rtc = embassy_rp::pac::RTC;
        rtc.irq_setup_0().modify(|w| w.set_match_ena(false));
    }
}
