use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

type DynError = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    dotenvy::dotenv().ok();
    let task = env::args().nth(1);
    let args = env::args().skip(2).collect::<Vec<_>>();

    match (task.as_deref(), args.as_slice()) {
        (Some("flash"), [binary]) => flash(binary)?,
        (Some("pygen"), _) => build_bindings()?,
        (Some("publish"), _) => publish()?,
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

flash <name>            flashes the firmware binary to the device
pygen                   generates the Python bindings
publish                 publishes the Python bindings to PyPI
"
    )
}

fn flash(binary: &str) -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);

    let defmt_log = env::var("DEFMT_LOG").unwrap_or_else(|_| "info".to_string());

    cmd.current_dir(firmware_dir())
        .env("DEFMT_LOG", defmt_log)
        .arg("build")
        .arg("--release")
        .arg("--target=thumbv7m-none-eabi")
        .arg(format!("--bin={}", binary));

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("Failed to build: {}", status).into());
    }

    let target_bin = project_root()
        .join("target")
        .join("thumbv7m-none-eabi")
        .join("release")
        .join(binary);

    let mut cmd = Command::new("probe-rs");

    cmd.arg("run").arg("--chip=STM32F103C8").arg(target_bin);

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("Failed to flash: {}", status).into());
    }

    Ok(())
}

fn publish() -> Result<(), DynError> {
    build_stubs()?;

    let mut cmd = Command::new("uv");

    cmd.current_dir(bindings_dir());
    cmd.arg("run")
        .arg("maturin")
        .arg("publish")
        .arg("--no-sdist");

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("Failed to build bindings: {}", status).into());
    }
    Ok(())
}

fn build_bindings() -> Result<(), DynError> {
    build_stubs()?;

    let mut cmd = Command::new("uv");

    cmd.current_dir(bindings_dir());
    cmd.arg("run").arg("maturin").arg("develop");

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("Failed to build bindings: {}", status).into());
    }
    Ok(())
}

fn build_stubs() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);

    cmd.current_dir(project_root());

    cmd.arg("run")
        .arg("-p")
        .arg("rustpill_clients")
        .arg("--release")
        .arg("--bin=stub_gen");

    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("Failed to build type stubs: {}", status).into());
    }
    Ok(())
}

fn firmware_dir() -> PathBuf {
    project_root().join("firmware")
}

fn bindings_dir() -> PathBuf {
    project_root().join("rustpill_clients")
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
