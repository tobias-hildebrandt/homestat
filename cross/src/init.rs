use crate::Irqs;
use cyw43::Control;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::Peri;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::Pio;
use log::info;
use static_cell::StaticCell;

/// Set of [`embassy_rp::Peripherals`] needed to initialize firmware and pins.
pub struct InitPins {
    pub pin23: Peri<'static, PIN_23>,
    pub pin24: Peri<'static, PIN_24>,
    pub pin25: Peri<'static, PIN_25>,
    pub pin29: Peri<'static, PIN_29>,
    pub pio0: Peri<'static, PIO0>,
    pub dma_ch: Peri<'static, DMA_CH0>,
}

/// Initializes the firmware and pins, spawns firmware task.
pub async fn init_fw_and_pins(
    spawner: Spawner,
    periphs: InitPins,
) -> (Stack<'static>, &'static mut Control<'static>) {
    // 43439a0 wifi firmware
    let fw = unsafe { core::slice::from_raw_parts(0x101c1000 as *const u8, 0x3ccea) };
    // "Country Locale Matrix"
    // https://www.infineon.com/assets/row/public/documents/30/96/infineon-wi-fi-glossary-software-en.pdf
    let clm = unsafe { core::slice::from_raw_parts(0x101fe000 as *const u8, 0x1290) };

    info!("first bytes of fw: {:x?}", &fw[0..5]);
    info!("first bytes of clm: {:x?}", &clm[0..5]);

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
    static CYM43_STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = CYM43_STATE.init(cyw43::State::new());
    let (net_device, control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner
        .spawn(cyw43_task(runner))
        .expect("unable to spawn cyw43 task");

    // move control to static storage
    static CONTROL: StaticCell<Control> = StaticCell::new();
    let control = CONTROL.uninit().write(control);

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;
    control.gpio_set(0, true).await;

    let config = Config::dhcpv4(Default::default());
    // Use static IP configuration instead of DHCP
    //let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
    //    address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
    //    dns_servers: Vec::new(),
    //    gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
    //});

    // Generate random seed
    let seed = getrandom::u64().expect("unable to generate random network seed");

    info!("network seed: {seed}");

    // Init network stack
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    spawner
        .spawn(net_task(runner))
        .expect("unable to spawn net task");

    (stack, control)
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}
