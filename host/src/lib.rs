use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{ERROR_PATH, PingEndpoint, WireError},
};
use protocol::GetUniqueIdEndpoint;
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
            |d| d.serial_number() == Some("2137"),
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

    pub async fn get_id(&self) -> Result<u128, RustpillError<Infallible>> {
        let id = self.client.send_resp::<GetUniqueIdEndpoint>(&()).await?;
        let mut padded_id = [0u8; 16];
        padded_id[..12].copy_from_slice(&id);
        Ok(u128::from_le_bytes(padded_id))
    }
}

impl Default for RustpillClient {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn read_line() -> String {
    tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        line
    })
    .await
    .unwrap()
}
