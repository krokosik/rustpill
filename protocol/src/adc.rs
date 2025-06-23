use postcard_rpc::{TopicDirection, endpoints, topics};

#[cfg(feature = "use-std")]
#[cfg(feature = "use-std")]

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy                | RequestTy                            | ResponseTy            | Path              |
    | ----------                | ---------                            | ----------            | ----              |
    | GetUniqueIdEndpoint       | ()                                   | [u8; 12]              | "unique_id/get"   |
    | GetAdcValEndpoint         | ()                                   | u16                   | "adc_val/get"     | //what in path?
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
