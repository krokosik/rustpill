use std::{path::Path, str::Utf8Error};

use postcard_rpc::{host_client::HostClient, standard_icd::WireError};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::*;

use crate::{
    common::{BoardError, BoardResult, connect_to_board},
    flash::flash_binary,
};
use macros::blocking_async;

use protocol::minimal::*; // Change minimal to your protocol module

/// This class communicates with Bluepill Rust firmware. You can pass a serial number to the
/// constructor to connect to a specific device. If no port is passed, it will try to connect to the first
/// available device by product string.
#[gen_stub_pyclass]
#[pyclass]
pub struct MinimalClient {
    client: HostClient<WireError>,
}

#[blocking_async]
#[gen_stub_pymethods]
#[pymethods]
impl MinimalClient {
    #[new]
    #[pyo3(signature = (serial_number = None))]
    async fn new(serial_number: Option<&str>) -> BoardResult<Self> {
        let client = connect_to_board(USB_DEVICE_NAME, serial_number).await?;
        Ok(Self { client })
    }

    #[staticmethod]
    /// Flash the firmware to the board.
    /// This function will use the `probe-rs` tool to flash the firmware binary to the board.
    fn flash() -> PyResult<()> {
        let filename = Path::new(file!())
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            pyo3::exceptions::PyChildProcessError::new_err(
                "Mismatch between host filename and binary name. Use the flash_binary function with correct binary name.",
            )
        })?;
        flash_binary(filename)?;
        Ok(())
    }

    /// Close the connection to the board.
    fn close(&self) {
        self.client.close();
    }

    /// Check if the connection to the board is closed.
    fn is_connected(&self) -> bool {
        !self.client.is_closed()
    }

    /// Get the serial number of the board.
    /// The ID is a 92-bit number, which is padded to 128 bits with zeros.
    ///
    /// :return: The serial number of the board.
    async fn get_serial_number(&self) -> BoardResult<String, Utf8Error> {
        let id = self.client.send_resp::<GetUniqueIdEndpoint>(&()).await?;
        let id = str::from_utf8(&id).map_err(BoardError::Endpoint)?;
        Ok(id.to_owned())
    }
}
