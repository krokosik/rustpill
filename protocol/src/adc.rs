use postcard_rpc::{TopicDirection, endpoints, topics};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "use-std")]
use pyo3::prelude::*;
#[cfg(feature = "use-std")]
use pyo3_stub_gen::derive::*;

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy                | RequestTy     | ResponseTy        | Path              |
    | ----------                | ---------     | ----------        | ----              |
    | GetUniqueIdEndpoint       | ()            | [u8; 12]          | "unique_id/get"   |
    | SetPWMDutyEndpoint        | u16           | ()                | "heater/set_pwm"  |
    | GetPidvalsEndpoint        | ()            | Pidvals           | "pid/get_pidvals" |
    | HeaterDisableEndpoint     | ()            | ()                | "heater/disable"  |
    | HeaterEnableEndpoint      | ()            | ()                | "heater/enable"   |
    | SetPidConstsEndpoint      | [f32;2]       | ()                | "pid/set_consts"  |
    | PidResetEndpoint          | ()            | ()                | "pid/reset"       |
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
pub struct Pidvals {
    pub setpoint: u16,
    pub adc_val: u16,
    pub error_val: i16,
    pub err_corr: f32,
    pub kp: f32,
    pub ki: f32,
    pub dt: u64,
    pub prev_clk: u64,
    pub is_on: bool,
    pub max_int_val: f32,
    pub int_sum: f32,
}
