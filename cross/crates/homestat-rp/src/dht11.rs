use embassy_executor::Spawner;
use embassy_rp::{
    Peri,
    gpio::{Flex, Pin, Pull},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::{Duration, Instant, Timer, WithTimeout};
use homestat_wire::{
    ChecksumError, Number, ReadError, Reading, WireMessage, WireMessageDisplay, WithTimestamp,
};
use log::{error, info};

/// Initializes DHT11 pin and spawns task.
pub fn spawn_dht11(spawner: Spawner, pin: Peri<'static, impl Pin>) {
    let flex = Flex::new(pin);

    spawner
        .spawn(dht11_task(flex))
        .expect("unable to spawn DHT task");
}

const NUM_BITS: usize = 40;

// TODO: replace with single-slot channel?
pub(crate) static READING: Mutex<ThreadModeRawMutex, Option<WireMessage>> = Mutex::new(None);

/// DHT11 task
#[embassy_executor::task]
async fn dht11_task(mut pin: Flex<'static>) {
    loop {
        let current_reading = Dht11Reader::try_read(&mut pin).await;
        // always set high after attempted reading
        pin.set_high();

        let timestamp = Instant::now().as_micros();

        let new_reading = WithTimestamp {
            micros: timestamp,
            inner: current_reading,
        };

        // log
        match new_reading.inner.is_ok() {
            true => info!("{}", WireMessageDisplay(&new_reading)),
            false => error!("{}", WireMessageDisplay(&new_reading)),
        };

        // update our mutex
        {
            let mut reading = READING.lock().await;
            *reading = Some(new_reading);
        }

        Timer::after_secs(1).await;
    }
}

struct Dht11Reader;

impl Dht11Reader {
    // TODO: after sending start signal, just capture state of pin for (50+80)*40 us = 5.2ms, then
    // analyze afterwards? instead of relying on rising and falling edges to trigger next state

    // TODO: do everything synchronously to avoid executor scheduling lag?
    async fn try_read(pin: &mut Flex<'_>) -> Result<Reading, ReadError> {
        // set high
        pin.set_as_output();
        pin.set_high();

        // must wait at least 1 second before any communication
        Timer::after_millis(1500).await;

        // set low for at least 18 MILLI SECONDS! NOT MICRO SECONDS
        pin.set_low();
        Timer::after_millis(20).await;

        // set high
        pin.set_high();

        // prepare for read
        pin.set_pull(Pull::None);
        pin.set_as_input();

        // DHT sets low after 20-40 us, though we might miss falling edge
        pin.wait_for_low()
            .with_timeout(Duration::from_micros(45))
            .await
            .map_err(|_| ReadError::StartLowTimeout)?;

        // DHT stays low 80 us, then goes high
        pin.wait_for_rising_edge()
            .with_timeout(Duration::from_micros(85))
            .await
            .map_err(|_| ReadError::StartRisingTimeout)?;

        // DHT stays high 80 us, then goes low and starts data transmission
        pin.wait_for_falling_edge()
            .with_timeout(Duration::from_micros(85))
            .await
            .map_err(|_| ReadError::StartFallingTimeout)?;

        let mut high_durations = [0u64; NUM_BITS];

        for (bit_number, duration) in high_durations.iter_mut().enumerate() {
            // DHT starts low

            // DHT stays low for 50 ms, then goes high
            pin.wait_for_rising_edge()
                .with_timeout(Duration::from_micros(55))
                .await
                .map_err(|_| ReadError::DataRisingTimeout { bit: bit_number })?;

            // take timestmap after rising edge
            let high_time = Instant::now().as_micros();

            // DHT stays high EITHER 26-28us or 70um, then goes low
            // falling edge detection in 26us might be too fast? so we just wait for low
            pin.wait_for_low()
                .with_timeout(Duration::from_micros(75))
                .await
                .map_err(|_| ReadError::DataFallingTimeout { bit: bit_number })?;

            // take timestamp after falling edge
            let low_time = Instant::now().as_micros();

            // calculate how long the ping was high for
            *duration = low_time - high_time;
        }

        // set high again
        pin.set_high();
        pin.set_as_output();

        // try parsing the timings
        Ok(Self::parse_durations(high_durations)?)
    }

    fn parse_durations(durations: [u64; NUM_BITS]) -> Result<Reading, ChecksumError> {
        let mut bits = [false; NUM_BITS];
        for index in 0..NUM_BITS {
            bits[index] = duration_to_bit(durations[index]);
        }

        // SAFETY: unwrap OK since we never exceed NUM_BITS
        let humidity_whole = bits_to_u8(&bits[0..8].try_into().unwrap());
        let humidity_tenths = bits_to_u8(&bits[8..16].try_into().unwrap());
        let temp_whole = bits_to_u8(&bits[16..24].try_into().unwrap());
        let temp_tenths = bits_to_u8(&bits[24..32].try_into().unwrap());
        let checksum = bits_to_u8(&bits[32..40].try_into().unwrap());

        let expected_checksum = humidity_whole
            .overflowing_add(humidity_tenths)
            .0
            .overflowing_add(temp_whole)
            .0
            .overflowing_add(temp_tenths)
            .0;

        if expected_checksum != checksum {
            Err(ChecksumError {
                expected: expected_checksum,
                actual: checksum,
            })
        } else {
            Ok(Reading {
                temperature: Number {
                    whole: temp_whole,
                    tenths: temp_tenths,
                },
                humidity: Number {
                    whole: humidity_whole,
                    tenths: temp_tenths,
                },
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
