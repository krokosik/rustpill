use std::{convert::Infallible, fmt::Debug};

use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr, SchemaError},
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

    log::info!(
        "Connected to board. Protocol schemas:\n{:?}",
        client.get_schema_report().await?
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

    log::info!("Initialized board client");

    Ok(client)
}

#[derive(Debug)]
pub enum BoardError<E: Debug> {
    Comms(HostErr<WireError>),
    Protocol(SchemaError<WireError>),
    #[allow(dead_code)]
    Endpoint(E),
    InvalidData(String),
}

impl<E: Debug> From<HostErr<WireError>> for BoardError<E> {
    fn from(value: HostErr<WireError>) -> Self {
        Self::Comms(value)
    }
}

impl<E: Debug> From<SchemaError<WireError>> for BoardError<E> {
    fn from(value: SchemaError<WireError>) -> Self {
        Self::Protocol(value)
    }
}

impl<E: Debug> Into<PyErr> for BoardError<E> {
    fn into(self) -> PyErr {
        match self {
            BoardError::Comms(err) => {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Comms error: {:?}", err))
            }
            BoardError::Protocol(err) => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Protocol error: {:?}", err),
            ),
            BoardError::Endpoint(err) => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Endpoint error: {:?}", err),
            ),
            BoardError::InvalidData(msg) => {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid data: {}", msg))
            }
        }
    }
}
