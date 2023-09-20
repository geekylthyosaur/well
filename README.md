# Well
Wayland compositor

> A Wayland compositor is a program that takes the output from different applications and combines them into a single image that is displayed on your screen.

## Getting started
### Build
1. [Install Rust](https://www.rust-lang.org/tools/install).
1. Install dependencies
   ```sh
   # On Fedora
   dnf install wayland-devel libxkbcommon-devel
   ```
1. Clone this repository `git clone https://github.com/geekylthyosaur/well.git`.
1. Build using `cargo build --release`.
1. Get binary from `./target/release/well`.

## Configuration
### Configuration file
`well` is configured in `Lua` using the built-in runtime. `Lua` runs only once to evaluate the configuration file, so there is no performance overhead.

The configuration file used is located at `$XDG_CONFIG_HOME/well/config.lua` (or `$XDG_CONFIG_HOME/well.lua`). In release builds, if it does not exists, default configuration will be automatically written to `$XDG_CONFIG_HOME/well/config.lua`. In case of any errors, except for Lua ones, the default configuration from [examples/config.lua](https://github.com/geekylthyosaur/well/blob/main/examples/config.lua) will be used.

### Configuration options
See [examples/config.lua](https://github.com/geekylthyosaur/well/blob/main/examples/config.lua).
