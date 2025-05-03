#![cfg_attr(not(feature = "use-std"), no_std)]

use postcard_rpc::{TopicDirection, endpoints, topics};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "use-std")]
use pyo3::prelude::*;
#[cfg(feature = "use-std")]
use pyo3_stub_gen::derive::*;

endpoints! {
    list = SERVO_ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy                | RequestTy                            | ResponseTy            | Path              |
    | ----------                | ---------                            | ----------            | ----              |
    | PingX2Endpoint            | u32                                  | u32                   | "pingx2"          |
    | GetUniqueIdEndpoint       | ()                                   | [u8; 12]              | "unique_id/get"   |
    | ConfigureChannel          | (PwmChannel, ServoChannelConfig)     | ()                    | "servo/channel"   |
    | GetServoConfig            | ()                                   | ServoConfig           | "servo/config"    |
    | SetFrequencyEndpoint      | u32                                  | ()                    | "servo/frequency" |
}

topics! {
    list = TOPICS_IN_LIST;
    direction = TopicDirection::ToServer;
    | TopicTy                   | MessageTy     | Path              |
    | -------                   | ---------     | ----              |
}

topics! {
    list = TOPICS_OUT_LIST;
    direction = TopicDirection::ToClient;
    | TopicTy                   | MessageTy     | Path              | Cfg                           |
    | -------                   | ---------     | ----              | ---                           |
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq, Clone, Copy)]
pub enum PwmChannel {
    Channel1 = 0,
    Channel2 = 1,
    Channel3 = 2,
    Channel4 = 3,
}

impl TryFrom<u8> for PwmChannel {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(PwmChannel::Channel1),
            2 => Ok(PwmChannel::Channel2),
            3 => Ok(PwmChannel::Channel3),
            4 => Ok(PwmChannel::Channel4),
            _ => Err(value),
        }
    }
}

#[cfg_attr(feature = "use-std", gen_stub_pyclass, pyclass(get_all, set_all))]
#[derive(Serialize, Deserialize, Schema, Debug, Default, PartialEq, Clone)]
pub struct ServoChannelConfig {
    pub min_angle_duty_cycle: u16,
    pub max_angle_duty_cycle: u16,
    pub current_duty_cycle: u16,
    pub enabled: bool,
}

#[cfg_attr(feature = "use-std", gen_stub_pyclass, pyclass(get_all, set_all))]
#[derive(Serialize, Deserialize, Schema, Debug, Default, PartialEq, Clone)]
pub struct ServoConfig {
    pub servo_frequency: u32,
    pub max_duty_cycle: u16,
    pub channels: [ServoChannelConfig; 4],
}
