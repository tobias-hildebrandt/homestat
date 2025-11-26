# HomeStat (WIP)

A personal project for gathering data about my home.

Data is captured on a Raspberry Pi Pico W and sent to a server, where it
is processed and stored.

## Development and Deployment

### Layout
The project directory layout is based off of [this post][ferrous-cargo-layout].
In short, the code for the Pico is contained in its own cargo workspace
in `/cross`, and the rest of the code is just like a normal Rust workspace.

The various helper scripts follow the [xtask pattern][xtask]. See
[`crates/xtask`](crates/xtask).

### Dependencies for xtasks
in your PATH:
- `curl`
- [`picotool`][picotool]
- `tio` (or `cu` or `screen` or some other serial monitor)

### Set up Environment Variables for Secrets
(I use [direnv][direnv])
```sh
# .envrc
export WIFI_SSID="your ssid here"
export WIFI_PASSWORD="your wifi password here"
export SERVER_IP="your server ip here"
export SERVER_PORT="your server port here"
```

### Download and Flash Firmware to Pico W
- `cargo xtask download-firmware`
- `cargo xtask flash-firmware`

### Build and Upload to Pico W
- `cargo xtask build-pico`
- `cargo xtask upload-pico`

### Connect to Serial Interface for Logs
(choose one)
- `cu -s 57600 -l /dev/ttyACM0`
  - ~. to quit
- `tio -b 57600 -t /dev/ttyACM0`
  - CTRL+T Q to quit

## License
Unless otherwise noted, this project is released under AGPLv3+.

Note: `cargo xtask download-firmware` downloads firmware blobs from
[Infineon][infineon-gh] to `misc/firmware/`.
These are nonfree files under the Permissive Binary License 1.0.
Their license is downloaded along with the blobs.


## TODO
- [ ] embedded
  - [x] serial logging
  - [x] dht11
  - [x] wifi
  - [x] embassy framework
    - [ ] migrate away from async?
  - [ ] on-board temp measurement
  - [ ] sleep mode
  - [ ] flash memory for boot count?
  - [ ] setup USB before spawning task
  - [x] macro for firmware flash slices
  - [ ] macro, config file for pins
- [ ] server
  - [x] parse messages from probes
  - [ ] store in database
  - [ ] contact external APIs?
    - [ ] enedis/EDF, etc?
  - [ ] grafana as code?
  - [ ] integrate with [home assistant][home-assistant]?
- [ ] encrypt communication
- [ ] hardware
  - [x] dht11 communication
  - [ ] battery
  - [ ] 3d printed case
- [ ] cargo-xtask for simpler build/run commands
  - [x] firmware management
    - [x] download/clean
    - [x] flash
  - [x] build and upload to pico
    - [ ] add device flags
    - [ ] add cargo passthrough flags
  - [ ] tests
  - [ ] run server
  - [ ] envrc management?


<!-- links -->

[ferrous-cargo-layout]: https://ferrous-systems.com/blog/test-embedded-app
[home-assistant]: https://github.com/home-assistant/core
[firmware-license]: /misc/firmware/LICENSE-permissive-binary-license-1.0.txt
[picotool]: https://github.com/raspberrypi/picotool
[direnv]: https://direnv.net/
[xtask]: https://github.com/matklad/cargo-xtask
[infineon-gh]: https://github.com/Infineon/wifi-host-driver/tree/latest-v3.X/WiFi_Host_Driver/resources
