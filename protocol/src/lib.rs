#![cfg_attr(not(feature = "use-std"), no_std)]

use postcard_rpc::{TopicDirection, endpoints, topics};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

// ---

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy                | RequestTy     | ResponseTy            | Path              |
    | ----------                | ---------     | ----------            | ----              |
    | PingX2Endpoint            | u32           | u32                   | "pingx2"          |
    | GetUniqueIdEndpoint       | ()            | [u8; 12]              | "unique_id/get"   |
    | SetAngleEndpoint          | SetAngle      | ()                    | "servo/set_angle" |
    | GetAngleEndpoint          | ()            | u8                    | "servo/get_angle" |
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
pub struct SetAngle {
    pub angle: u8,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct GetAngle {
    pub angle: u8,
}
