mod request;
mod response;

pub use self::{request::*, response::*};
use serde::{Deserialize, Serialize};

pub const VERSION: Version = (0, 0);

pub type Version = (u32, u32);

bitflags::bitflags! {
    #[derive(Default, Deserialize, Serialize)]
    pub struct Features: u32 {
        const BASIC = 0b00000000;
        const TARGET = 0b00000001;
    }
}
