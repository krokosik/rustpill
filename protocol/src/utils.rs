use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

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
