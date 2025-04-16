use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{ERROR_PATH, PingEndpoint, WireError},
};
use protocol::{
    AccelRange, BadPositionError, GetUniqueIdEndpoint, Rgb8, SetAllLedEndpoint,
    SetSingleLedEndpoint, SingleLed, StartAccel, StartAccelerationEndpoint,
    StopAccelerationEndpoint,
};
use std::convert::Infallible;

pub struct RustpillClient {
    pub client: HostClient<WireError>,
}

#[derive(Debug)]
pub enum RustpillError<E> {
    Comms(HostErr<WireError>),
    Endpoint(E),
}

impl<E> From<HostErr<WireError>> for RustpillError<E> {
    fn from(value: HostErr<WireError>) -> Self {
        Self::Comms(value)
    }
}

trait FlattenErr {
    type Good;
    type Bad;
    fn flatten(self) -> Result<Self::Good, RustpillError<Self::Bad>>;
}

impl<T, E> FlattenErr for Result<T, E> {
    type Good = T;
    type Bad = E;
    fn flatten(self) -> Result<Self::Good, RustpillError<Self::Bad>> {
        self.map_err(RustpillError::Endpoint)
    }
}

// ---

impl RustpillClient {
    pub fn new() -> Self {
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

    pub async fn ping(&self, id: u32) -> Result<u32, RustpillError<Infallible>> {
        let val = self.client.send_resp::<PingEndpoint>(&id).await?;
        Ok(val)
    }

    pub async fn get_id(&self) -> Result<u64, RustpillError<Infallible>> {
        let id = self.client.send_resp::<GetUniqueIdEndpoint>(&()).await?;
        Ok(id)
    }
}

impl Default for RustpillClient {
    fn default() -> Self {
        Self::new()
    }
}
