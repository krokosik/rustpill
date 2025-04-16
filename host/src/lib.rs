use pyo3::prelude::*;

mod servo;

use servo::ServoClient;

#[pymodule]
fn rustpill_clients(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ServoClient>()?;
    Ok(())
}
