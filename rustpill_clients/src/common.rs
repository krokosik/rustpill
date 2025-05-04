use std::convert::Infallible;

use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{ERROR_PATH, LoggingTopic, WireError},
};
use pyo3::prelude::*;

#[cfg(target_os = "linux")]
use std::path::PathBuf;

pub async fn connect_to_board(
    port: Option<&str>,
) -> Result<HostClient<WireError>, BoardError<Infallible>> {
    let client = HostClient::new_raw_nusb(
        |d| {
            if port.is_some() {
                log::info!("Trying to connect to port {}", port.unwrap());

                #[cfg(target_os = "windows")]
                {
                    assert!(port.unwrap().starts_with("COM"));
                    format!("COM{}", d.port_number()) == port.unwrap()
                }

                #[cfg(target_os = "linux")]
                {
                    d.sysfs_path() == PathBuf::from(port.unwrap())
                }
            } else {
                log::info!("Trying to connect to first available device");
                d.product_string() == Some("bluepill-servo")
            }
        },
        ERROR_PATH,
        8,
        VarSeqKind::Seq2,
    );

    let mut logsub = client.subscribe_multi::<LoggingTopic>(64).await.unwrap();

    // Spawn a background task to handle log messages
    pyo3_async_runtimes::tokio::get_runtime().spawn(async move {
        log::info!("Starting log subscription");
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

    Ok(client)
}

#[derive(Debug)]
pub enum BoardError<E> {
    Comms(HostErr<WireError>),
    Endpoint(E),
    InvalidData(String),
}

impl<E> From<HostErr<WireError>> for BoardError<E> {
    fn from(value: HostErr<WireError>) -> Self {
        Self::Comms(value)
    }
}

impl<E> Into<PyErr> for BoardError<E> {
    fn into(self) -> PyErr {
        match self {
            BoardError::Comms(err) => {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{:?}", err))
            }
            BoardError::Endpoint(_) => {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Endpoint error")
            }
            BoardError::InvalidData(msg) => PyErr::new::<pyo3::exceptions::PyValueError, _>(msg),
        }
    }
}
