# Rust for STM32 Bluepill

This is a WIP implementation/exploration of using modern Rust technologies for more convenient programming of Bluepill MCUs, with potential for porting to different boards with minimal effort. The idea is to have an easy to use Rust client for performing remote procedure calls (RPC) with the MCU, which is also what the `packet` package does in the original `bluepill-dds` project. There are 4 core dependencies used in the project and it might be useful to get to know some of them:

- **Embassy** is a Rust framework for MCUs providing an async executor for bare metal software. The programs are structured into tasks, with abstractions/HALs for STM32, RP2040, NRF or ESP families
    - [The Embedded Rust Book](https://docs.rust-embedded.org/book/) a great collection of information about using Rust for embedded. For development, I recommend to at least read about the basic abstractions regarding peripherals, `#[no_std]`, tips for C developers and C interoperability, if you used C before.
    - [Embassy website and Embassy Book](https://embassy.dev/) most of the stuff above applies to pure embedded Rust, but we have Embassy as well, which provides a very convenient set of abstractions. Checking out their examples on Github is also beneficial.
    - [Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/) both firmware and host client rely on async code, some basics about how it works in Rust are also of use.
- **Postcard RPC** a framework for efficient, type-safe communication with the MCU. Their [repo](https://github.com/jamesmunns/postcard-rpc?tab=readme-ov-file) is probably the best place to start, with a very good overview that also explains the project structure.
- **PyO3** a library to generate Python bindings from Rust code [guide](https://pyo3.rs/).
- **probe-rs** used for flashing firmware and debugging code running on MCU.

Other interesting resources:
- [Bluepill HAL docs](https://docs.embassy.dev/embassy-stm32/git/stm32f103c8/index.html)
- [Bluepill PAC docs](https://docs.embassy.dev/stm32-metapac/git/stm32f103c8/index.html)
- [Bluepill datasheet](https://www.st.com/resource/en/datasheet/stm32f103c8.pdf)
- [Workbook for Embedded Workshops](https://embedded-trainings.ferrous-systems.com/preparations)
- [The Embedonomicon](https://docs.rust-embedded.org/embedonomicon/preface.html)

## Abstractions

In comparison to C, there is much more abstraction offered by the Rust ecosystem, in order to deliver convenience and safety. The `embassy` framework offers its own `embassy-stm32` HAL as well as a crate with structs generated based on actual hardware - PAC.

![Crates](./docs/crates.png)

You can read more [here](https://docs.rust-embedded.org/book/start/registers.html) (note that this book uses a different HAL and PAC). Usually, you should rely on the HAL offered abstractions, and resort to `embassy-stm32::pac` or `cortex-m` crate when necessary. The best point to start is looking at the documentation, linked above.

## Setup

1. Install Rust via [rustup](https://www.rust-lang.org/tools/install) and `probe-rs` version `0.28.0` via:
```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/probe-rs/probe-rs/releases/download/v0.28.0/probe-rs-tools-installer.ps1 | iex"
```
2. Install the `uv` package manager for building Python bindings: https://docs.astral.sh/uv/getting-started/installation/
3. If developing via WSL or Dev Container, you need to bind and attach the USB Bus via [usbipd-win](https://learn.microsoft.com/en-us/windows/wsl/connect-usb), but everything should work on Windows

## Workspace

This repo is structued into 4 distinct crates that are managed together with a Cargo workspace. To run commands in one of them, just use the `-p` flag, for example to compile the firmware:

```
cargo build -p firmware --release
```

Unfortunately, due to limitations of cargo, the workspace uses nightly features for multi target integration. Some things do not work perfectly, so we use `xtasks` for running commands rather than pure cargo.
Here is a brief description of each crate:

- **protocol** here we define statically typed message formats, endpoints and topics (streamed data). It is built automatically when building the dependent host and firmware crates.
- **firmware** MCU codes with different functionalities. Each binary is one firmware and you select which one you want to flash by adding `--bin <name>` to the flash command.
- **host** the host crate where Rust clients are defined and then Python bindings are generated on top of them.
- **xtasks** Rust written build scripts, kind of like `make` [more info](https://github.com/matklad/cargo-xtask)
- **macros** For convenience, custom Rust macros (codegen tools) are stored in a separate crate.

## How to start

1. Connect the Bluepill board via ST-LINK, but do not connect the 3V pin (bend it to the side)
2. Connect the Bluepill board to the PC via USB.
3. Flash it with `cargo xtask flash minimal` or any other binary
4. Build Python bindings with the Maturin build tool: `cargo xtask pygen`
5. Test the commands in the `test.py` file. Make sure you use the `uv` created local virtual environment.

## New firmware

1. Create copies of the `minimal.rs` files in `protocol`, `firmware` and `host` crates.
2. Rename them to the **same** name that will be the name of your binary from now on.
3. Initialize the modules. Add new statements for your binary in [host](host/src/hosts/mod.rs) and [protocol](protocol/src/lib.rs)
4. Rename your host client struct and add it to the [Python module](host/src/lib.rs)
5. Update the `protocol` imports in `firmware` and `host`
6. Now you can start developing. Create a communication schema in `protocol` and then proceed to implementing logic, handlers in `firmware` and callers in `host`

## Debugging

Install the `probe-rs` VS Code extension and set breakpoints in the code. Go to the firmware binary code file, for example `minimal.rs` and run the `probe-rs binary` debugger or simply press `F5`.

There are some issues to be ironed out in the config or the tool itself though:
- https://github.com/probe-rs/probe-rs/issues/3045

If you want to play with raw binaries (for example use the ST-Link companion software or analyze size), you can use some custom utilities from [cargo-binutils](https://github.com/rust-embedded/cargo-binutils):
```shell
# Install needed only once
cargo install cargo-binutils
# Example commands include size, nm, objdump, strip...
cargo <cmd> -p firmware --bin <binary> --release
```

## Wishlist

- expand the firmware and port more Cube code
- another crate for RP Pico firmware
- create a global defmt logger that sends data via a topic 
- fix the `firmware` runner, which is ignored when using `forced-target`, i.e. find a way to make the Cargo workspace work nicer. [cargo issue](https://github.com/rust-lang/cargo/issues/14833). Another option might be to exclude firmware from workspace and use `linkedProjects` in Rust Analyzer
- test multiple connected boards scenario

## Asyncio

Currently we run a Tokio event loop inside the Rust binary, which is responsible for all the Rust async logic. As our Python code does not really rely on `asyncio`, all the exposed Python binding are synchronous and are converted into such by wrapping every Rust async function body in 

```rust
pyo3_async_runtimes::tokio::get_runtime().block_on(async move { /* code */ }) 
```

This is taken care of by the `blocking_async` macro, making all of the Python clients blocking. In the future (hehe), we might want to have non-blocking calls as well, which might be very useful for streams of data. It is also worth noting that IPython supports top-level await. However, the state of async in PyO3 is not ready for that yet:

- tracking issue https://github.com/PyO3/pyo3/issues/1632
- async constructors not supported https://github.com/PyO3/pyo3/issues/5176
- type stub generator ignores async https://github.com/Jij-Inc/pyo3-stub-gen
- there is no convenient API for streams https://github.com/PyO3/pyo3-async-runtimes/issues/35, https://github.com/awestlake87/pyo3-asyncio/issues/17 and tracking issue
- the `pyo3-asyncio` and `pyo3-async-runtimes` seem dead. Development seems to be continuing in the main repo, under `experimental-async` flag.

In conclusion it might be better to wait until we really really need async in Python. It is doable, but it might be better to just wait for the design to stabilize.