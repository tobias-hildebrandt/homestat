# HomeStat

## TODO
- [ ] server
- [ ] flash memory for boot count?
- [ ] on-board temp measurement
- [ ] sleep mode
- [ ] hardware
  - [ ] battery
- [ ] cargo-xtask for simpler build command?

## Running RPI Pico W version
### build and upload
`cargo run --release --manifest-path cross/Cargo.toml --config cross/.cargo/config.toml`
or
`sh -c "cd cross; cargo run --release"`
or
`env -C cross cargo run --release`

### connect to serial
`cu -s 57600 -l /dev/ttyACM0`
(~. to quit)
or
`tio -b 57600 -t -l --log-file $LOG_FILE /dev/ttyACM0`
(CTRL+T Q to quit)

## Cargo workspaces & VSCode project layout
https://ferrous-systems.com/blog/test-embedded-app
