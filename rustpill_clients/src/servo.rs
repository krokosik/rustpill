use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{ERROR_PATH, LoggingTopic, WireError},
};
use protocol::{GetUniqueIdEndpoint, PingX2Endpoint};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::*;
use std::convert::Infallible;

/// This class communicates with Bluepill Servo Rust firmware.
#[gen_stub_pyclass]
#[pyclass]
pub struct ServoClient {
    client: HostClient<WireError>,
}

#[derive(Debug)]
pub enum ServoError<E> {
    Comms(HostErr<WireError>),
    Endpoint(E),
}

impl<E> From<HostErr<WireError>> for ServoError<E> {
    fn from(value: HostErr<WireError>) -> Self {
        Self::Comms(value)
    }
}

impl<E> Into<PyErr> for ServoError<E> {
    fn into(self) -> PyErr {
        match self {
            ServoError::Comms(err) => {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{:?}", err))
            }
            ServoError::Endpoint(_) => {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Endpoint error")
            }
        }
    }
}

// ---
#[gen_stub_pymethods]
#[pymethods]
impl ServoClient {
    #[new]
    #[tokio::main]
    pub async fn new() -> Self {
        let client = HostClient::new_raw_nusb(
            |d| d.product_string() == Some("bluepill-servo"),
            ERROR_PATH,
            8,
            VarSeqKind::Seq2,
        );
        let mut logsub = client.subscribe_multi::<LoggingTopic>(64).await.unwrap();

        // Spawn a background task to handle log messages
        tokio::spawn(async move {
            loop {
                match logsub.recv().await {
                    Ok(log) => {
                        log::info!("FIRMWARE: {}", log);
                    }
                    Err(e) => {
                        log::error!("Log subscription error: {:?}", e);
                        break;
                    }
                }
            }
        });

        Self { client }
    }

    pub async fn wait_closed(&self) {
        self.client.wait_closed().await;
    }

    /// Send a ping to the board and return the response.
    /// A number is sent to the device, and the device is expected to return the same number times 2.
    pub async fn pingx2(&self, id: u32) -> Result<u32, ServoError<Infallible>> {
        let val = self.client.send_resp::<PingX2Endpoint>(&id).await?;
        Ok(val)
    }

    /// Get the unique ID of the board.
    /// The ID is a 92-bit number, which is padded to 128 bits with zeros.
    pub async fn get_id(&self) -> Result<u128, ServoError<Infallible>> {
        let id = self.client.send_resp::<GetUniqueIdEndpoint>(&()).await?;
        let mut padded_id = [0u8; 16];
        padded_id[..12].copy_from_slice(&id);
        Ok(u128::from_le_bytes(padded_id))
    }

    /// Set the angle of the servo.
    /// The angle is a number between 0 and 180.
    pub async fn set_angle(&self, angle: u8) -> Result<(), ServoError<Infallible>> {
        self.client
            .send_resp::<protocol::SetAngleEndpoint>(&protocol::SetAngle { angle })
            .await?;
        Ok(())
    }

    /// Get the angle of the servo.
    /// The angle is a number between 0 and 180.
    pub async fn get_angle(&self) -> Result<u8, ServoError<Infallible>> {
        let angle = self
            .client
            .send_resp::<protocol::GetAngleEndpoint>(&())
            .await?;
        Ok(angle)
    }
}

impl Default for ServoClient {
    fn default() -> Self {
        Self::new()
    }
}
