# Rust for STM32 Bluepill

## Setup

1. Connect the Bluepill board via ST-LINK
2. Install Rust via [rustup](https://www.rust-lang.org/tools/install) and [probe-rs](https://probe.rs/docs/getting-started/probe-setup)
3. If developing via WSL or Dev Container, you need to bind and attach the USB Bus via [usbipd-win](https://learn.microsoft.com/en-us/windows/wsl/connect-usb)
5. Check compilation with `cargo build --bin <binary file> --release`
6. Upload to the board with `cargo run --bin <binary file> --release`
7. After first flash, you will have to manually press RESET button, try to upload, release RESET and upload again, unless you use an idle task like in `servo.rs`
