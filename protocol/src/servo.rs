use postcard_rpc::{TopicDirection, endpoints, topics};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "use-std")]
use pyo3::prelude::*;
#[cfg(feature = "use-std")]
use pyo3_stub_gen::derive::*;

use crate::utils::PwmChannel;

endpoints! {
    list = SERVO_ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy                | RequestTy                            | ResponseTy            | Path              |
    | ----------                | ---------                            | ----------            | ----              |
    | GetUniqueIdEndpoint       | ()                                   | [u8; 24]              | "unique_id/get"   |
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
