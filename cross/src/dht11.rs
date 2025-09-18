use core::fmt::Display;

use embassy_executor::Spawner;
use embassy_rp::{
    Peri,
    gpio::{Flex, Pin, Pull},
};
use embassy_time::{Instant, Timer};
use homestat_wire::{Humidity, Reading, Temperature, WholeAndDecimal};
use log::{info, warn};
use serde::Serialize;

/// Initializes DHT11 pin and spawns task.
pub fn spawn_dht11(spawner: Spawner, pin: Peri<'static, impl Pin>) {
    let flex = Flex::new(pin);

    spawner.spawn(sender_task(flex)).unwrap();
}

const NUM_BITS: usize = 40;
const ENCODING_BUFFER_LEN: usize = 64;

/// DHT11 task
#[embassy_executor::task]
async fn sender_task(mut flex: Flex<'static>) {
    let mut buffer = [0u8; ENCODING_BUFFER_LEN];

    buffer.fill(0);
    loop {
        flex.set_as_output();
        flex.set_high();

        // must wait at least 1 second before any communication
        Timer::after_secs(2).await;

        // low for at least 18 MILLI SECONDS! NOT MICRO SECONDS
        flex.set_low();
        Timer::after_millis(20).await;

        // high for 30 us
        flex.set_high();
        Timer::after_micros(30).await;

        // set low, prepare for read
        flex.set_low();
        flex.set_pull(Pull::None);
        flex.set_as_input();

        // low 80 us
        // high 80 us
        flex.wait_for_falling_edge().await;

        let mut high_durations = [0u64; NUM_BITS];

        for duration in high_durations.iter_mut() {
            // ~50 ms low
            flex.wait_for_rising_edge().await;
            let high_time = Instant::now().as_micros();
            // high 27-28us or 70us
            flex.wait_for_falling_edge().await;
            let low_time = Instant::now().as_micros();
            *duration = low_time - high_time;
        }

        flex.set_high();
        flex.set_as_output();

        let reading = Dht11Reading::from_durations(high_durations);
        if let Some(valid) = reading {
            info!("reading: {}", valid);
            let code_res = postcard::to_slice(&valid, &mut buffer);
            match code_res {
                Ok(coded) => info!("reading coded: {:?}", coded),
                Err(e) => log::error!("encoding failed: {:?}", e),
            }
        } else {
            warn!("checksum failure")
        };

        Timer::after_secs(1).await;
    }
}

#[derive(Debug, Serialize)]
struct Dht11Reading(Reading);

impl Display for Dht11Reading {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}.{}degC, {}.{}%RH",
            self.0.temperature.0.integer,
            self.0.temperature.0.decimal,
            self.0.humidity.0.integer,
            self.0.humidity.0.decimal,
        )
    }
}

impl Dht11Reading {
    fn from_durations(durations: [u64; NUM_BITS]) -> Option<Self> {
        let mut bits = [false; NUM_BITS];
        for index in 0..NUM_BITS {
            bits[index] = duration_to_bit(durations[index]);
        }

        let mut chunks = bits.chunks(8);
        let integral_humidity = bits_to_u8(chunks.next().unwrap().try_into().unwrap());
        let decimal_humidity = bits_to_u8(chunks.next().unwrap().try_into().unwrap());
        let integral_temp = bits_to_u8(chunks.next().unwrap().try_into().unwrap());
        let decimal_temp = bits_to_u8(chunks.next().unwrap().try_into().unwrap());
        let checksum = bits_to_u8(chunks.next().unwrap().try_into().unwrap());

        if integral_humidity + decimal_humidity + integral_temp + decimal_temp != checksum {
            None
        } else {
            Some(Self(Reading {
                temperature: Temperature(WholeAndDecimal {
                    integer: integral_temp,
                    decimal: decimal_temp,
                }),
                humidity: Humidity(WholeAndDecimal {
                    integer: integral_humidity,
                    decimal: decimal_temp,
                }),
            }))
        }
    }
}

/// Convert a microsecond high duration to a bit.
fn duration_to_bit(duration: u64) -> bool {
    duration > 45
}

/// Convert bits to a u8.
///
/// "The sensor sends higher data bit first."
fn bits_to_u8(bits: &[bool; 8]) -> u8 {
    ((bits[0] as u8) << 7)
        | ((bits[1] as u8) << 6)
        | ((bits[2] as u8) << 5)
        | ((bits[3] as u8) << 4)
        | ((bits[4] as u8) << 3)
        | ((bits[5] as u8) << 2)
        | ((bits[6] as u8) << 1)
        | (bits[7] as u8)
}
