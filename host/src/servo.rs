use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{ERROR_PATH, WireError},
};
use protocol::{GetUniqueIdEndpoint, PingX2Endpoint};
use pyo3::prelude::*;
use std::convert::Infallible;

#[pyclass]
pub struct ServoClient {
    pub client: HostClient<WireError>,
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
        Self { client }
    }

    pub async fn wait_closed(&self) {
        self.client.wait_closed().await;
    }

    pub async fn pingx2(&self, id: u32) -> Result<u32, ServoError<Infallible>> {
        let val = self.client.send_resp::<PingX2Endpoint>(&id).await?;
        Ok(val)
    }

    pub async fn get_id(&self) -> Result<u128, ServoError<Infallible>> {
        let id = self.client.send_resp::<GetUniqueIdEndpoint>(&()).await?;
        let mut padded_id = [0u8; 16];
        padded_id[..12].copy_from_slice(&id);
        Ok(u128::from_le_bytes(padded_id))
    }

    pub async fn set_angle(&self, angle: u8) -> Result<(), ServoError<Infallible>> {
        self.client
            .send_resp::<protocol::SetAngleEndpoint>(&protocol::SetAngle { angle })
            .await?;
        Ok(())
    }

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
