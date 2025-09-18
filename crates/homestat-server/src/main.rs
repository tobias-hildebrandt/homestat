use homestat_wire::{Humidity, Reading, Temperature, WholeAndDecimal};
use tracing::info;

fn main() {
    tracing_subscriber::fmt().init();

    let mut buffer = [0u8; 2048];
    let reading = Reading {
        temperature: Temperature(WholeAndDecimal {
            integer: 1,
            decimal: 2,
        }),
        humidity: Humidity(WholeAndDecimal {
            integer: 3,
            decimal: 4,
        }),
    };

    let encoded = postcard::to_slice(&reading, &mut buffer);

    info!("{:?}", encoded);
}
