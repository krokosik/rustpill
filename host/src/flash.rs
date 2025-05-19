use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

use pyo3::prelude::*;
use pyo3_stub_gen_derive::gen_stub_pyfunction;

const PROBE_RS_VERSION: &str = "0.28.0";

/// Checks if "probe-rs" is available in PATH. If not, prompts the user and
/// attempts installation.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn check_probe_rs() {
    if which("probe-rs").is_some() {
        println!("Probe-rs is installed.");
        return;
    } else {
        println!("Probe-rs is not installed.");
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
            println!("Installation not supported on this platform.");
            exit(1);
        };

        match status {
            Ok(s) if s.success() => {
                if which("probe-rs").is_some() {
                    println!("Probe-rs installed successfully.");
                    return;
                } else {
                    println!("Probe-rs installation did not complete correctly.");
                    exit(1);
                }
            }
            Ok(s) => {
                println!("Installation failed with exit code: {}", s);
                exit(1);
            }
            Err(e) => {
                println!("Installation failed: {}", e);
                exit(1);
            }
        }
    }
}

/// Lists the binaries available in the "assets" directory relative to this source file.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn list_binaries() {
    // Determine the base directory. Assuming the assets folder is located in the same
    // directory as this source file.
    let base_dir = get_assets_dir();
    println!("Available binaries:");
    match fs::read_dir(&base_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    println!("- {}", file_name);
                }
            }
        }
        Err(e) => {
            println!("Failed to read assets directory: {}", e);
            exit(1);
        }
    }
}

/// Flashes the specified binary by calling "cargo-flash" tool on the given binary.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn flash_binary(binary_name: &str) {
    check_probe_rs();

    let base_dir = get_assets_dir();
    let binary_path = base_dir.join(binary_name);

    println!("Starting flash of binary: {}", binary_path.display());

    let status = Command::new("cargo-flash")
        .args(&["--chip", "STM32F103C8", "--path"])
        .arg(binary_path)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("Flashing completed successfully.");
        }
        Ok(s) => {
            println!("Flashing failed with exit code: {}", s);
            exit(1);
        }
        Err(e) => {
            println!("Flashing failed: {}", e);
            exit(1);
        }
    }
}

/// Helper function to locate the assets directory.
/// Assumes that the assets directory is in the same folder as this source file.
fn get_assets_dir() -> PathBuf {
    // The env!("CARGO_MANIFEST_DIR") returns the directory containing Cargo.toml.
    // Adjust the relative path as necessary. Here we assume assets is in "host/src/assets".
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .join("host")
        .join("src")
        .join("assets")
}

/// Simple implementation of the Unix 'which' command.
/// Returns Some(path) if the binary is found in PATH.
fn which(cmd: &str) -> Option<PathBuf> {
    if cmd.contains(std::path::MAIN_SEPARATOR) {
        let path = Path::new(cmd);
        if path.is_file() {
            return Some(path.to_path_buf());
        }
        return None;
    }
    if let Ok(paths) = env::var("PATH") {
        for path in env::split_paths(&paths) {
            let full_path = path.join(cmd);
            if full_path.is_file() {
                return Some(full_path);
            }
            // On Windows, executables might have extensions like .exe, .bat, etc.
            #[cfg(windows)]
            {
                let exts = env::var("PATHEXT").unwrap_or_default();
                for ext in exts.split(';') {
                    let full_path_ext = full_path.with_extension(ext.trim_start_matches('.'));
                    if full_path_ext.is_file() {
                        return Some(full_path_ext);
                    }
                }
            }
        }
    }
    None
}
