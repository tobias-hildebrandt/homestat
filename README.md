# HomeStat

## Cargo workspaces & VSCode project layout
https://ferrous-systems.com/blog/test-embedded-app

## Running RPI Pico W version
cargo run --release --manifest-path cross/Cargo.toml --config cross/.cargo/config.toml \
    && sleep 1 \
    && cu -s 57600 -l /dev/ttyACM0

