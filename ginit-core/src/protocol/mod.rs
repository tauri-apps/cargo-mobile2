mod request;
mod response;

pub use self::{request::*, response::*};

pub const VERSION: Version = (0, 0);

pub type Version = (u32, u32);
