#![cfg_attr(not(feature = "use-std"), no_std)]

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
    | EndpointTy                | RequestTy                            | ResponseTy            | Path              |
    | ----------                | ---------                            | ----------            | ----              |
    | GetUniqueIdEndpoint       | ()                                   | [u8; 12]              | "unique_id/get"   |
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
pub struct EmptyConfig {}
