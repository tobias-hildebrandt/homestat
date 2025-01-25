use core::fmt::Display;

use embassy_executor::Spawner;
use embassy_rp::{
    Peripheral,
    gpio::{Flex, Pin, Pull},
};
use embassy_time::{Instant, Timer};
use log::{info, warn};

/// Initializes DHT11 pin and spawns task.
pub fn spawn_dht11(spawner: &Spawner, pin: impl Peripheral<P = impl Pin> + 'static) {
    let flex = Flex::new(pin);

    spawner.spawn(sender_task(flex)).unwrap();
}

const NUM_BITS: usize = 40;

/// DHT11 task
#[embassy_executor::task]
async fn sender_task(mut flex: Flex<'static>) {
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

        let reading = Reading::from_durations(high_durations);
        if let Some(valid) = reading {
            info!("reading: {}", valid);
        } else {
            warn!("checksum failure")
        };

        Timer::after_secs(1).await;
    }
}

#[derive(Debug)]
struct Reading {
    integral_humidity: u8,
    decimal_humidity: u8,
    integral_temp: u8,
    decimal_temp: u8,
}

impl Display for Reading {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}.{}degC, {}.{}%RH",
            self.integral_temp, self.decimal_temp, self.integral_humidity, self.decimal_humidity
        )
    }
}

impl Reading {
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
            Some(Self {
                integral_humidity,
                decimal_humidity,
                integral_temp,
                decimal_temp,
            })
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
