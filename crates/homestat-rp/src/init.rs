use crate::Irqs;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::Pio;
use static_cell::StaticCell;

/// Set of [`embassy_rp::Peripherals`] needed to initialize firmware and pins.
pub struct InitPins {
    pub pin23: PIN_23,
    pub pin24: PIN_24,
    pub pin25: PIN_25,
    pub pin29: PIN_29,
    pub pio0: PIO0,
    pub dma_ch: DMA_CH0,
}

/// Initializes the firmware and pins, spawns firmware task.
pub async fn init_fw_and_pins(spawner: &Spawner, periphs: InitPins) -> cyw43::Control<'_> {
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

    // configure pins
    let pwr = Output::new(periphs.pin23, Level::Low);
    let cs = Output::new(periphs.pin25, Level::High);
    let mut pio = Pio::new(periphs.pio0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        periphs.pin24,
        periphs.pin29,
        periphs.dma_ch,
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

    control
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}
