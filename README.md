# Rust for STM32 Bluepill

This is a WIP implementation/exploration of using modern Rust technologies for more convenient programming of Bluepill MCUs, with potential for porting to different boards with minimal effort. The idea is to have an easy to use Rust client for performing remote procedure calls (RPC) with the MCU, which is also what the `packet` package does in the original `bluepill-dds` project. There are 4 core dependencies used in the project and it might be useful to get to know some of them:

- **Embassy** is a Rust framework for MCUs providing an async executor for bare metal software. The programs are structured into tasks, with abstractions/HALs for STM32, RP2040, NRF or ESP families
    - [The Embedded Rust Book](https://docs.rust-embedded.org/book/) a great collection of information about using Rust for embedded. For development, I recommend to at least read about the basic abstractions regarding peripherals, `#[no_std]`, tips for C developers and C interoperability, if you used C before.
    - [Embassy website and Embassy Book](https://embassy.dev/) most of the stuff above applies to pure embedded Rust, but we have Embassy as well, which provides a very convenient set of abstractions. Checking out their examples on Github is also beneficial.
    - [Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/) both firmware and host client rely on async code, some basics about how it works in Rust are also of use.
- **Postcard RPC** a framework for efficient, type-safe communication with the MCU. Their [repo](https://github.com/jamesmunns/postcard-rpc?tab=readme-ov-file) is probably the best place to start, with a very good overview that also explains the project structure.
- **PyO3** a library to generate Python bindings from Rust code [guide](https://pyo3.rs/).
- **probe-rs** used for flashing firmware and debugging code running on MCU. I haven't explored it that much yet.

## Setup

1. Install Rust via [rustup](https://www.rust-lang.org/tools/install) and [probe-rs](https://probe.rs/docs/getting-started/probe-setup) for flashing
2. Install the `uv` package manager for building Python bindings: https://docs.astral.sh/uv/getting-started/installation/
3. If developing via WSL or Dev Container, you need to bind and attach the USB Bus via [usbipd-win](https://learn.microsoft.com/en-us/windows/wsl/connect-usb), but everything should work on Windows

## Workspace

This repo is structued into 4 distinct crates that are managed together with a Cargo workspace. To run commands in one of them, just use 

```
cargo -p <package> <command>
```

Unfortunately, due to limitations of cargo, the workspace uses nightly features for multi target integration. Some things do not work perfectly, so we use `xtasks` for running commands rather than pure cargo.
Here is a brief description of each crate:

- **protocol** here we define statically typed message formats, endpoints and topics (streamed data). It is built automatically when building the dependent host and firmware crates.
- **firmware** MCU codes with different functionalities. Each binary is one firmware and you select which one you want to flash by adding `--bin <name>` to the flash command.
- **rustpill_clients** the host crate where Rust clients are defined and then Python bindings are generated on top of them.
- **xtasks** Rust written build scripts, kind of like `make` [more info](https://github.com/matklad/cargo-xtask)

## How to start

1. Connect the Bluepill board via ST-LINK
2. Flash it with `cargo xtask flash servo` or any other binary
3. Build Python bindings with the Maturin build tool: `cargo xtask pygen`
4. Test the commands in the `test.py` file. Make sure you use the `uv` created local virtual environment.

## Wishlist

- expand the firmware and port more Cube code
- figure out how to pass sender to handler, to support easy logging
- fix the `firmare` runner, which is ignored when using `forced-target`, i.e. find a way to make the Cargo workspace work nicer. [cargo issue](https://github.com/rust-lang/cargo/issues/14833)