use pyo3::{ffi::c_str, prelude::*};

mod common;
mod flash;
mod servo;

use pyo3_stub_gen::define_stub_info_gatherer;
use servo::ServoClient;

/// This module hosts Python wrappers for communicating with Bluepill Rust firmware.
#[pymodule]
fn rustpill_clients(m: &Bound<'_, PyModule>) -> PyResult<()> {
    Python::with_gil(|py| {
        py.run(
            c_str!(
                "
import logging
if not logging.getLogger().hasHandlers():
    FORMAT = \"%(levelname)s %(name)s %(message)s\"
    logging.basicConfig(format=FORMAT)
    logging.getLogger().setLevel(logging.INFO)
"
            ),
            None,
            None,
        )
    })?;

    pyo3_log::init();

    m.add_function(wrap_pyfunction!(flash::check_probe_rs, m)?)?;
    m.add_function(wrap_pyfunction!(flash::flash_binary, m)?)?;
    m.add_class::<ServoClient>()?;
    Ok(())
}

define_stub_info_gatherer!(stub_info);
