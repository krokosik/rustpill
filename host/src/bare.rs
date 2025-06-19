use macros::blocking_async;
use postcard_rpc::{host_client::HostClient, standard_icd::WireError};
use protocol::{EmptyConfig, GetUniqueIdEndpoint};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::*;
use std::convert::Infallible;

use crate::{
    common::{BoardError, connect_to_board},
    flash::flash_binary,
};

// const STM32_PWM_RESOLUTION_BITS: u8 = 16;

/// This class communicates with Bluepill Servo Rust firmware. You can pass a serial number to the
/// constructor to connect to a specific device. If no port is passed, it will try to connect to the first
/// available device by product string.
#[gen_stub_pyclass]
#[pyclass]
pub struct Client {
    client: HostClient<WireError>,
    #[pyo3(get)]
    config: EmptyConfig,
}

#[blocking_async]
#[gen_stub_pymethods]
#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (serial_number = None))]
    async fn new(serial_number: Option<&str>) -> Result<Self, BoardError<Infallible>> {
        let client = connect_to_board(serial_number).await?;

        let config = client.send_resp::<protocol::GetServoConfig>(&()).await?;
        log::info!("Servo config: {:?}", config);

        Ok(Self { client, config })
    }

    #[staticmethod]
    /// Flash the servo firmware to the board.
    /// This function will use the `probe-rs` tool to flash the firmware binary to the board.
    fn flash() -> PyResult<()> {
        flash_binary("servo")?;
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
    async fn get_serial_number(&self) -> Result<u128, BoardError<Infallible>> {
        let id = self.client.send_resp::<GetUniqueIdEndpoint>(&()).await?;
        let mut padded_id = [0u8; 16];
        padded_id[..12].copy_from_slice(&id);
        Ok(u128::from_le_bytes(padded_id))
    }
}
