use pyo3::prelude::*;

mod servo;

use pyo3_stub_gen::define_stub_info_gatherer;
use servo::ServoClient;

#[pymodule]
fn rustpill_clients(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ServoClient>()?;
    Ok(())
}

define_stub_info_gatherer!(stub_info);
