# Rust for STM32 Bluepill

## Setup

1. Connect the Bluepill board via ST-LINK
2. Bind and attach the USB Bus via [usbipd-win](https://learn.microsoft.com/en-us/windows/wsl/connect-usb)
3. Inside WSL, add `udev` rules according to this [tutorial](https://probe.rs/docs/getting-started/probe-setup/#linux%3A-udev-rules)
4. Open the Dev Container via VS Code.
5. Check compilation with `cargo build --bin <binary file> --release`
6. Upload to the board with `cargo run --bin <binary file> --release`
