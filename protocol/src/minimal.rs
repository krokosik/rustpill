use postcard_rpc::{TopicDirection, endpoints, topics};

pub const USB_DEVICE_NAME: &'static str = "bluepill-minimal";

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy                | RequestTy                            | ResponseTy            | Path              |
    | ----------                | ---------                            | ----------            | ----              |
    | GetUniqueIdEndpoint       | ()                                   | [u8; 24]              | "unique_id/get"   |
    | StartDefmtLoggingEndpoint | ()                                   | ()                    | "defmt/start"     |
    | StopDefmtLoggingEndpoint  | ()                                   | bool                  | "defmt/stop"      |
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
    | TopicTy                   | MessageTy         | Path              | Cfg   |
    | -------                   | ---------         | ----              | ---   |
    | DefmtLoggingTopic         | (u8, [u8; 32])    | "defmt/log"       |       |
}
