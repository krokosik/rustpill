use std::sync::OnceLock;

use pyo3::prelude::*;

mod servo;

use servo::ServoClient;
use tokio::runtime::Runtime;

static RT: OnceLock<Runtime> = OnceLock::new();

#[pymodule]
fn rustpill_clients(m: &Bound<'_, PyModule>) -> PyResult<()> {
    RT.get_or_init(|| Runtime::new().expect("Failed to create Tokio runtime"));

    m.add_class::<ServoClient>()?;
    Ok(())
}
