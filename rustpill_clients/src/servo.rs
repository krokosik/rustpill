use postcard_rpc::{
    header::VarSeqKind,
    host_client::{HostClient, HostErr},
    standard_icd::{ERROR_PATH, LoggingTopic, WireError},
};
use protocol::{GetUniqueIdEndpoint, PingX2Endpoint, PwmChannel};
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
    InvalidData(String),
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
            ServoError::InvalidData(msg) => PyErr::new::<pyo3::exceptions::PyValueError, _>(msg),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl ServoClient {
    #[new]
    #[pyo3(signature = (port = None))]
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
    /// :param channel: The channel to set the servo on (1-4), corresponding to PWM channels on pins PB6-PB9.
    /// :param angle: The angle to set the servo to (0-180).
    pub fn set_angle(&self, channel: u8, angle: u8) -> Result<(), ServoError<Infallible>> {
        let channel = PwmChannel::try_from(channel)
            .map_err(|_| ServoError::InvalidData("Invalid channel".to_string()))?;
        if angle > 180 {
            return Err(ServoError::InvalidData("Invalid angle".to_string()));
        }

        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            self.client
                .send_resp::<protocol::SetAngleEndpoint>(&(channel as PwmChannel, angle))
                .await
        })?;
        Ok(())
    }

    /// Get the angle of the servo on channel 1-4.
    ///
    /// :return: The angle of the servo (0-180).
    pub fn get_angle(&self, channel: u8) -> Result<u8, ServoError<Infallible>> {
        let channel = PwmChannel::try_from(channel)
            .map_err(|_| ServoError::InvalidData("Invalid channel".to_string()))?;

        let angle = pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            self.client
                .send_resp::<protocol::GetAngleEndpoint>(&channel)
                .await
        })?;
        Ok(angle)
    }

    /// Configure the servo channel.
    /// This function sets the minimum and maximum duty cycle for the servo channel,
    /// which corresponds to the minimum and maximum angle. Leave arguments as None to use
    /// to not change them on device.
    ///
    /// :param channel: The channel to configure (1-4).
    /// :param enabled: Whether the channel is enabled or not. Board boots with all channels disabled.
    /// :param current_duty_cycle: The current duty cycle of the channel. Set to 0 on boot.
    /// :param min_angle_duty_cycle: The minimum duty cycle for the channel. By default uses values corresponding to a pulse width of 500us.
    /// :param max_angle_duty_cycle: The maximum duty cycle for the channel. By default uses values corresponding to a pulse width of 2500us.
    #[pyo3(signature = (
        channel,
        enabled = None,
        current_duty_cycle = None,
        min_angle_duty_cycle = None,
        max_angle_duty_cycle = None,
    ))]
    pub fn configure_channel(
        &self,
        channel: u8,
        enabled: Option<bool>,
        current_duty_cycle: Option<u16>,
        min_angle_duty_cycle: Option<u16>,
        max_angle_duty_cycle: Option<u16>,
    ) -> Result<(), ServoError<Infallible>> {
        let channel = PwmChannel::try_from(channel)
            .map_err(|_| ServoError::InvalidData("Invalid channel".to_string()))?;
        let config = protocol::ServoChannelConfigRqst {
            min_angle_duty_cycle,
            max_angle_duty_cycle,
            current_duty_cycle,
            enabled,
        };
        if min_angle_duty_cycle > max_angle_duty_cycle {
            return Err(ServoError::InvalidData(
                "min_angle_duty_cycle > max_angle_duty_cycle".to_string(),
            ));
        }

        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            self.client
                .send_resp::<protocol::ConfigureChannel>(&((channel, config)))
                .await
        })?;
        Ok(())
    }

    /// Get the servo configuration.
    /// This function returns the current configuration of the servo channels as a JSON string.
    /// It needs to be formatted using `json.loads()` in Python.
    /// :return: The servo configuration as a JSON string.
    pub fn get_config(&self) -> Result<String, ServoError<Infallible>> {
        let config = pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            self.client.send_resp::<protocol::GetServoConfig>(&()).await
        })?;

        Ok(serde_json::to_string(&config)
            .map_err(|e| ServoError::InvalidData(format!("Failed to serialize config: {}", e)))?)
    }

    /// Set the frequency of the PWM signal.
    /// This function sets the frequency of the PWM signal for all channels.
    /// The frequency is set in Hz, default on device is 50 Hz. Note that all
    /// channels will be disabled when the frequency is changed, as the max duty cycle
    /// changes and settings need to be readjusted.
    /// :param frequency: The frequency to set in Hz.
    pub fn set_frequency(&self, frequency: u32) -> Result<(), ServoError<Infallible>> {
        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            self.client
                .send_resp::<protocol::SetFrequencyEndpoint>(&frequency)
                .await
        })?;
        Ok(())
    }
}

impl Default for ServoClient {
    fn default() -> Self {
        Self::new(None)
    }
}
