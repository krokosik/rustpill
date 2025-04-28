#![cfg_attr(not(feature = "use-std"), no_std)]

use postcard_rpc::{TopicDirection, endpoints, topics};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "use-std")]
use pyo3::prelude::*;
#[cfg(feature = "use-std")]
use pyo3_stub_gen::derive::*;

// ---

endpoints! {
    list = SERVO_ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy                | RequestTy     | ResponseTy            | Path              |
    | ----------                | ---------     | ----------            | ----              |
    | PingX2Endpoint            | u32           | u32                   | "pingx2"          |
    | GetUniqueIdEndpoint       | ()            | [u8; 12]              | "unique_id/get"   |
    | SetAngleEndpoint          | u8            | ()                    | "servo/set_angle" |
    | GetAngleEndpoint          | ()            | u8                    | "servo/get_angle" |
    | SetServoMinEndpoint       | u32           | ()                    | "servo/set_min"   |
    | SetServoMaxEndpoint       | u32           | ()                    | "servo/set_max"   |
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

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
#[cfg_attr(feature = "use-std", gen_stub_pyclass, pyclass)]
pub struct ServoPwmConfig {
    pub min_servo_duty_cycle: u16,
    pub max_servo_duty_cycle: u16,
    pub servo_frequency: u32,
    pub max_duty_cycle: u16,
    pub current_duty_cycle: u16,
}
