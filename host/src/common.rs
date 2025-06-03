use std::{convert::Infallible, fmt::Debug};

use anyhow::anyhow;
use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr, IoClosed, SchemaError},
    standard_icd::{ERROR_PATH, WireError},
};
use pyo3::prelude::*;

use crate::{flash::get_binary, log::run_decoder};

pub async fn connect_to_board(
    product_string: &str,
    serial_number: Option<&str>,
) -> Result<HostClient<WireError>, BoardError<Infallible>> {
    if serial_number.is_some() {
        log::info!("Connecting to device with S/N: {}", serial_number.unwrap());
    } else {
        log::info!("Connecting to first available device");
    }

    let client = HostClient::new_raw_nusb(
        |d| {
            let res = {
                if serial_number.is_some() {
                    d.serial_number() == serial_number
                } else {
                    d.product_string() == Some(product_string)
                }
            };
            // Sadly HostClient doesn't expose the DeviceInfo struct
            if res {
                let version = d.device_version();
                let patch = version & 0x000F;
                let minor = (version & 0x00F0) >> 4;
                let major = ((version & 0x0F00) >> 8) + 10 * ((version & 0xF000) >> 12);
                let version = format!("{major}.{minor}.{patch}");

                log::info!(
                    "Found device: {} v{} (SN: {})",
                    d.product_string().unwrap_or("Unknown"),
                    version,
                    d.serial_number().unwrap_or("N/A")
                );

                if version != env!("CARGO_PKG_VERSION") {
                    log::warn!(
                        "Device version {} does not match host version {}. Consider flashing using the client's `flash` command.",
                        version,
                        env!("CARGO_PKG_VERSION")
                    );
                }
            }
            res
        },
        ERROR_PATH,
        8,
        VarSeqKind::Seq2,
    );

    log::info!("Connected to servo board");

    let mut logsub = client
        .subscribe_multi::<protocol::DefmtLoggingTopic>(64)
        .await?;

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

    core::mem::drop(tokio::task::spawn(async move {
        let binary_path = get_binary("servo").unwrap();
        run_decoder(&binary_path, rx).await
    }));

    log::info!("Created log subscription");

    // Spawn a background task to handle log messages
    core::mem::drop(tokio::task::spawn(async move {
        log::info!("Starting log subscription");
        while let Ok((n, buf)) = logsub.recv().await {
            if tx.send(buf[..n as usize].to_vec()).is_err() {
                log::error!("Failed to send data to decoder");
                break;
            }
        }
    }));

    log::info!("Initialized board client");

    Ok(client)
}

#[derive(Debug)]
pub enum BoardError<E: Debug = Infallible> {
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

impl<E: Debug> From<IoClosed> for BoardError<E> {
    fn from(_value: IoClosed) -> Self {
        Self::Comms(HostErr::Closed)
    }
}

impl<E: Debug> From<BoardError<E>> for PyErr {
    fn from(val: BoardError<E>) -> Self {
        match val {
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

impl<E: Debug> From<u8> for BoardError<E> {
    fn from(value: u8) -> Self {
        BoardError::InvalidData(format!("PWM channel {} out of 1-4 range", value))
    }
}

pub type BoardResult<T, E = Infallible> = Result<T, BoardError<E>>;
