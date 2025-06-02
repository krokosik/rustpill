use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio, exit};
use std::thread::sleep;
use std::{env, time};

use pyo3::prelude::*;
use pyo3_stub_gen_derive::gen_stub_pyfunction;
use s3_utils::get_bucket;

const PROBE_RS_VERSION: &str = "0.28.0";

#[gen_stub_pyfunction]
#[pyfunction]
pub fn check_probe_rs() {
    if Command::new("probe-rs").arg("--version").status().is_ok() {
        log::info!("Probe-rs is installed.");
        return;
    } else {
        log::info!("Probe-rs is not installed.");
        let status = if cfg!(target_os = "windows") {
            let script = format!(
                "irm https://github.com/probe-rs/probe-rs/releases/download/v{version}/probe-rs-tools-installer.ps1 | iex",
                version = PROBE_RS_VERSION
            );
            Command::new("powershell")
                .args(&["-ExecutionPolicy", "Bypass", "-c", &script])
                .status()
        } else if cfg!(target_os = "linux") {
            // Use a shell to pipe the output of curl to sh.
            let cmd = format!(
                "curl --proto '=https' --tlsv1.2 -sSLf https://github.com/probe-rs/probe-rs/releases/download/v{version}/probe-rs-tools-installer.sh | sh",
                version = PROBE_RS_VERSION
            );
            Command::new("sh").arg("-c").arg(&cmd).status()
        } else {
            log::info!("Installation not supported on this platform.");
            exit(1);
        };

        match status {
            Ok(s) if s.success() => {
                if Command::new("probe-rs").arg("--version").status().is_ok() {
                    log::info!("Probe-rs installed successfully.");
                    return;
                } else {
                    log::info!("Probe-rs installation did not complete correctly.");
                    exit(1);
                }
            }
            Ok(s) => {
                log::info!("Installation failed with exit code: {}", s);
                exit(1);
            }
            Err(e) => {
                log::info!("Installation failed: {}", e);
                exit(1);
            }
        }
    }
}

#[gen_stub_pyfunction]
#[pyfunction]
pub fn flash_binary(binary_name: &str) -> PyResult<()> {
    check_probe_rs();

    let binary_path = env::temp_dir().join(binary_name);

    if !binary_path.exists() {
        log::info!("Downloading binary: {:?}", binary_name);

        let bucket = get_bucket().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to get S3 bucket: {}",
                e
            ))
        })?;

        let mut file = File::create(&binary_path)?;

        let mut binary = bucket
            .get_object(["stm32f103c8", env!("CARGO_PKG_VERSION"), binary_name].join("/"))
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to get object from S3: {}",
                    e
                ))
            })?;

        file.write_all(binary.bytes_mut())?;
        file.flush()?;
    }
    log::info!("Flashing binary: {}", binary_name);

    let mut cmd = Command::new("probe-rs");
    cmd.arg("download")
        .arg("--chip=STM32F103C8")
        .arg("--non-interactive")
        .arg("--disable-progressbars")
        .arg("--protocol")
        .arg("swd")
        .arg(&binary_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to spawn probe-rs: {}",
            e
        ))
    })?;

    // Capture and log stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => log::info!("probe-rs: {}", line),
                Err(e) => log::warn!("Error reading stdout: {}", e),
            }
        }
    }

    // Capture and log stderr
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => log::info!("probe-rs error: {}", line),
                Err(e) => log::warn!("Error reading stderr: {}", e),
            }
        }
    }

    let status = child.wait().map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to wait for probe-rs: {}",
            e
        ))
    })?;

    if !status.success() {
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to flash binary: {}",
            status
        )));
    }

    sleep(time::Duration::from_secs(1));

    log::info!("Resetting board after flashing...");

    Command::new("probe-rs")
        .arg("reset")
        .arg("--chip=STM32F103C8")
        .arg("--non-interactive")
        .arg("--protocol")
        .arg("swd")
        .status()
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to reset board: {}",
                e
            ))
        })?;

    sleep(time::Duration::from_secs(2));

    Ok(())
}
