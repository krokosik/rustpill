use std::str::Utf8Error;

use macros::blocking_async;
use postcard_rpc::{host_client::HostClient, standard_icd::WireError};
use protocol::heater::{
    GetPidvalsEndpoint, GetUniqueIdEndpoint, HeaterDisableEndpoint, HeaterEnableEndpoint,
    PidResetEndpoint, Pidvals, RecalcPIEndpoint, SetPISetpoint, SetPWMDutyEndpoint,
    SetPidConstsEndpoint,
};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::*;

use crate::{
    common::{BoardError, BoardResult, connect_to_board},
    flash::flash_binary,
};

/// This class communicates with Bluepill Rust firmware. You can pass a serial number to the
/// constructor to connect to a specific device. If no port is passed, it will try to connect to the first
/// available device by product string.
#[gen_stub_pyclass]
#[pyclass]
pub struct Client {
    client: HostClient<WireError>,
}

#[blocking_async]
#[gen_stub_pymethods]
#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (serial_number = None))]
    async fn new(serial_number: Option<&str>) -> BoardResult<Self> {
        let client = connect_to_board(serial_number).await?;

        Ok(Self { client })
    }

    #[staticmethod]
    /// Flash the servo firmware to the board.
    /// This function will use the `probe rs` tool to flash the firmware binary to the board.
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

    /*START FUNCTIONS FOR PYTHON CLIENT*/

    /// Get the serial number of the board.
    /// The ID is a 92 bit number, which is padded to 128 bits with zeros.
    ///
    /// :return: The serial number of the board.
    async fn get_serial_number(&self) -> BoardResult<String, Utf8Error> {
        let id = self.client.send_resp::<GetUniqueIdEndpoint>(&()).await?;
        let id = str::from_utf8(&id).map_err(BoardError::Endpoint)?;
        Ok(id.to_owned())
    }

    async fn disable_heater(&self) -> BoardResult<()> {
        self.client.send_resp::<HeaterDisableEndpoint>(&()).await?;
        Ok(())
    }
    async fn enable_heater(&self) -> BoardResult<()> {
        self.client.send_resp::<HeaterEnableEndpoint>(&()).await?;
        Ok(())
    }
    async fn set_heater_duty(&self, duty: u16) -> BoardResult<()> {
        self.client.send_resp::<SetPWMDutyEndpoint>(&duty).await?;
        Ok(())
    }
    async fn reset_pid(&self) -> BoardResult<()> {
        self.client.send_resp::<PidResetEndpoint>(&()).await?;
        Ok(())
    }
    async fn set_pid_consts(&self, kp: f32, ki: f32) -> BoardResult<()> {
        self.client
            .send_resp::<SetPidConstsEndpoint>(&[kp, ki])
            .await?;
        Ok(())
    }
    async fn get_pid_vals(&self) -> BoardResult<Pidvals> {
        let pidvals = self.client.send_resp::<GetPidvalsEndpoint>(&()).await?;
        Ok(pidvals)
    }
    async fn recalc_pi(&self) -> BoardResult<()> {
        self.client.send_resp::<RecalcPIEndpoint>(&()).await?;
        Ok(())
    }

    async fn set_setpoint(&self, val: u16) -> BoardResult<()> {
        self.client.send_resp::<SetPISetpoint>(&val).await?;
        Ok(())
    }
}
//END FUNCTIONS FOR PYTHON CLIENT
