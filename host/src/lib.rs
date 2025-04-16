use pyo3::prelude::*;

mod runtime;
mod servo;

use runtime::RT;
use servo::ServoClient;
use tokio::runtime::Runtime;

#[pymodule]
fn rustpill_clients(m: &Bound<'_, PyModule>) -> PyResult<()> {
    RT.get_or_init(|| Runtime::new().expect("Failed to create Tokio runtime"));

    m.add_class::<ServoClient>()?;
    Ok(())
}
