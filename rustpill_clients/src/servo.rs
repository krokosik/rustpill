use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{ERROR_PATH, LoggingTopic, WireError},
};
use protocol::{GetUniqueIdEndpoint, PingX2Endpoint};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::*;
use std::convert::Infallible;

#[cfg(target_os = "linux")]
use std::path::PathBuf;

/// This class communicates with Bluepill Servo Rust firmware. You can pass a port string to the
/// constructor to connect to a specific port. If no port is passed, it will try to connect to the first
/// available device by product string.
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

#[gen_stub_pymethods]
#[pymethods]
impl ServoClient {
    #[new]
    pub fn new(port: Option<&str>) -> Self {
        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            let client = HostClient::new_raw_nusb(
                |d| {
                    if port.is_some() {
                        #[cfg(target_os = "windows")]
                        {
                            assert!(port.unwrap().starts_with("COM"));
                            format!("COM{}", d.port_number()) == port.unwrap()
                        }

                        #[cfg(target_os = "linux")]
                        {
                            d.path().0 == PathBuf::from(port.unwrap())
                        }
                    } else {
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

            Self { client }
        })
    }

    pub fn wait_closed(&self) {
        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            self.client.wait_closed().await;
        });
    }

    /// Send a ping to the board and return the response.
    /// A number is sent to the device, and the device is expected to return the same number times 2.
    ///
    /// :param val: The number to send to the device.
    /// :return: The number returned by the device: val * 2.
    pub fn pingx2(&self, val: u32) -> Result<u32, ServoError<Infallible>> {
        let dbl_val = pyo3_async_runtimes::tokio::get_runtime()
            .block_on(async move { self.client.send_resp::<PingX2Endpoint>(&val).await })?;
        Ok(dbl_val)
    }

    /// Get the unique ID of the board.
    /// The ID is a 92-bit number, which is padded to 128 bits with zeros.
    ///
    /// :return: The unique ID of the board.
    pub fn get_id(&self) -> Result<u128, ServoError<Infallible>> {
        let id = pyo3_async_runtimes::tokio::get_runtime()
            .block_on(async move { self.client.send_resp::<GetUniqueIdEndpoint>(&()).await })?;
        let mut padded_id = [0u8; 16];
        padded_id[..12].copy_from_slice(&id);
        Ok(u128::from_le_bytes(padded_id))
    }

    /// Set the angle of the servo.
    ///
    /// :param angle: The angle to set the servo to (0-180).
    pub fn set_angle(&self, angle: u8) -> Result<(), ServoError<Infallible>> {
        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            self.client
                .send_resp::<protocol::SetAngleEndpoint>(&angle)
                .await
        })?;
        Ok(())
    }

    /// Get the angle of the servo.
    ///
    /// :return: The angle of the servo (0-180).
    pub fn get_angle(&self) -> Result<u8, ServoError<Infallible>> {
        let angle = pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            self.client
                .send_resp::<protocol::GetAngleEndpoint>(&())
                .await
        })?;
        Ok(angle)
    }

    /// Set the minimum duty cycle of the servo.
    ///
    /// :param min: The minimum duty cycle to set the servo to.
    pub fn set_servo_min(&self, min: u32) -> Result<(), ServoError<Infallible>> {
        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            self.client
                .send_resp::<protocol::SetServoMinEndpoint>(&min)
                .await
        })?;
        Ok(())
    }

    /// Set the maximum duty cycle of the servo.
    ///     
    /// :param max: The maximum duty cycle to set the servo to.
    pub fn set_servo_max(&self, max: u32) -> Result<(), ServoError<Infallible>> {
        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            self.client
                .send_resp::<protocol::SetServoMaxEndpoint>(&max)
                .await
        })?;
        Ok(())
    }

    /// Get the PWM configuration of the servo.
    ///
    /// :return: The PWM configuration of the servo.
    pub fn get_pwm_config(&self) -> Result<protocol::ServoPwmConfig, ServoError<Infallible>> {
        let config = pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            self.client
                .send_resp::<protocol::ServoPwmConfigEndpoint>(&())
                .await
        })?;
        Ok(config)
    }
}

impl Default for ServoClient {
    fn default() -> Self {
        Self::new(None)
    }
}
