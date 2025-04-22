use pyo3::prelude::*;

mod servo;

use pyo3_stub_gen::define_stub_info_gatherer;
use servo::ServoClient;

/// This module hosts Python wrappers for communicating with Bluepill Rust firmware.
#[pymodule]
fn rustpill_clients(m: &Bound<'_, PyModule>) -> PyResult<()> {
    pyo3_log::init();

    m.add_class::<ServoClient>()?;
    Ok(())
}

define_stub_info_gatherer!(stub_info);
