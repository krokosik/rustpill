use std::{path::Path, str::Utf8Error};

use macros::blocking_async;
use postcard_rpc::{host_client::HostClient, standard_icd::WireError};
use protocol::{servo::*, utils::PwmChannel};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::*;

use crate::{
    common::{BoardError, BoardResult, connect_to_board},
    flash::flash_binary,
};

const STM32_PWM_RESOLUTION_BITS: u8 = 16;

/// This class communicates with Bluepill Servo Rust firmware. You can pass a serial number to the
/// constructor to connect to a specific device. If no port is passed, it will try to connect to the first
/// available device by product string.
#[gen_stub_pyclass]
#[pyclass]
pub struct ServoClient {
    client: HostClient<WireError>,
    #[pyo3(get)]
    config: ServoConfig,
}

#[blocking_async]
#[gen_stub_pymethods]
#[pymethods]
impl ServoClient {
    #[new]
    #[pyo3(signature = (serial_number = None))]
    async fn new(serial_number: Option<&str>) -> BoardResult<Self> {
        let client = connect_to_board(USB_DEVICE_NAME, serial_number).await?;

        let config = client.send_resp::<GetServoConfig>(&()).await?;
        log::info!("Servo config: {:?}", config);

        Ok(Self { client, config })
    }

    #[staticmethod]
    /// Flash the servo firmware to the board.
    /// This function will use the `probe-rs` tool to flash the firmware binary to the board.
    fn flash() -> PyResult<()> {
        let filename = Path::new(file!())
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            pyo3::exceptions::PyChildProcessError::new_err(
                "Mismatch between host filename and binary name. Use the flash_binary function with correct binary name.",
            )
        })?;
        flash_binary(filename)?;
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

    /// Get the serial number of the board.
    /// The ID is a 92-bit number, which is padded to 128 bits with zeros.
    ///
    /// :return: The serial number of the board.
    async fn get_serial_number(&self) -> BoardResult<String, Utf8Error> {
        let id = self.client.send_resp::<GetUniqueIdEndpoint>(&()).await?;
        let id = str::from_utf8(&id).map_err(BoardError::Endpoint)?;
        Ok(id.to_owned())
    }

    /// Set the angle of the servo.
    ///
    /// :param channel: The channel to set the servo on (1-4), corresponding to PWM channels on pins PB6-PB9.
    /// :param angle: The angle to set the servo to (0-180).
    fn set_angle(&mut self, channel: u8, angle: u8) -> BoardResult<()> {
        PwmChannel::try_from(channel)?;
        if angle > 180 {
            return Err(BoardError::InvalidData("Invalid angle".to_string()));
        }

        let channel_config = &self.config.channels[channel as usize - 1];

        self.configure_channel(
            channel,
            Some(true),
            Some(self.angle_to_duty_cycle(
                angle,
                channel_config.min_angle_duty_cycle,
                channel_config.max_angle_duty_cycle,
            )),
            None,
            None,
        )?;
        Ok(())
    }

    /// Get the angle of the servo on channel 1-4.
    ///
    /// :return: The angle of the servo (0-180).
    fn get_angle(&self, channel: u8) -> BoardResult<u8> {
        let channel = PwmChannel::try_from(channel)?;

        let channel_config = &self.config.channels[channel as usize];
        let angle = self.duty_cycle_to_angle(
            channel_config.current_duty_cycle,
            channel_config.min_angle_duty_cycle,
            channel_config.max_angle_duty_cycle,
        );
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
    async fn configure_channel(
        &mut self,
        channel: u8,
        enabled: Option<bool>,
        current_duty_cycle: Option<u16>,
        min_angle_duty_cycle: Option<u16>,
        max_angle_duty_cycle: Option<u16>,
    ) -> BoardResult<()> {
        let channel = PwmChannel::try_from(channel)?;
        let channel_config = &mut self.config.channels[channel as usize];

        channel_config.enabled = enabled.unwrap_or(channel_config.enabled);
        channel_config.current_duty_cycle =
            current_duty_cycle.unwrap_or(channel_config.current_duty_cycle);
        channel_config.min_angle_duty_cycle =
            min_angle_duty_cycle.unwrap_or(channel_config.min_angle_duty_cycle);
        channel_config.max_angle_duty_cycle =
            max_angle_duty_cycle.unwrap_or(channel_config.max_angle_duty_cycle);

        let channel_config = channel_config.clone();

        self.client
            .send_resp::<ConfigureChannel>(&(channel, channel_config))
            .await?;
        Ok(())
    }

    /// Get the servo configuration.
    /// This function returns the current configuration of the servo channels.
    /// :return: The ServoConfig object.
    async fn update_config(&mut self) -> BoardResult<()> {
        let config = self.client.send_resp::<GetServoConfig>(&()).await?;
        self.config = config;
        Ok(())
    }

    /// Set the frequency of the PWM signal.
    /// This function sets the frequency of the PWM signal for all channels.
    /// The frequency is set in Hz, default on device is 50 Hz. Note that all
    /// channels will be disabled when the frequency is changed, as the max duty cycle
    /// changes and settings need to be readjusted.
    /// :param frequency: The frequency to set in Hz.
    async fn set_frequency(&mut self, frequency: u32) -> BoardResult<()> {
        self.client
            .send_resp::<SetFrequencyEndpoint>(&frequency)
            .await?;
        self.update_config()
    }

    fn angle_to_duty_cycle(&self, angle: u8, min_duty_cycle: u16, max_duty_cycle: u16) -> u16 {
        if angle > 180 {
            return max_duty_cycle;
        }
        let duty_cycle = ((angle as f32) / 180.0) * (max_duty_cycle - min_duty_cycle) as f32
            + min_duty_cycle as f32;
        duty_cycle.round() as u16
    }

    fn duty_cycle_to_angle(&self, duty_cycle: u16, min_duty_cycle: u16, max_duty_cycle: u16) -> u8 {
        if duty_cycle < min_duty_cycle {
            return 0;
        }
        if duty_cycle > max_duty_cycle {
            return 180;
        }
        let angle = ((duty_cycle - min_duty_cycle) as f32
            / (max_duty_cycle - min_duty_cycle) as f32)
            * 180.0;
        angle.round() as u8
    }

    /// Convert microseconds to duty cycle.
    fn us_to_duty_cycle(&self, us: u32) -> u16 {
        let frequency = self.config.servo_frequency as f32;
        let period = 1_000_000.0 / frequency;
        let us = us as f32;
        let resolution = 2.0f32.powi(STM32_PWM_RESOLUTION_BITS as i32);
        let duty_cycle = (us / period) * resolution;
        duty_cycle.round() as u16
    }
}
