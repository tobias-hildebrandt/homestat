use homestat_wire::{Number, Reading};
use tracing::info;

fn main() {
    tracing_subscriber::fmt().init();

    let mut buffer = [0u8; 2048];
    let reading = Reading {
        temperature: Number {
            whole: 1,
            tenths: 2,
        },
        humidity: Number {
            whole: 3,
            tenths: 4,
        },
    };

    let encoded = postcard::to_slice(&reading, &mut buffer);

    info!("{:?}", encoded);
}
