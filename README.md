# Well
Wayland compositor

> A Wayland compositor is a program that takes the output from different applications and combines them into a single image that is displayed on your screen.

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
