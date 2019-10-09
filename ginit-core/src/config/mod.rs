pub mod app_name;
mod empty;
mod shared;

pub use self::{empty::*, shared::*};
use crate::util;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

pub trait ConfigTrait: Debug + Serialize + Sized {
    type DefaultConfig: DefaultConfigTrait;

    type Raw: for<'r> Deserialize<'r> + Serialize;
    type Error: Debug + Display;
    fn from_raw(shared: Shared, raw: Option<Self::Raw>) -> Result<Self, Self::Error>;

    fn shared(&self) -> &Shared;

    fn insert_template_data(&self, key: &str, map: &mut bicycle::JsonMap) {
        map.insert(key, self);
        self.shared().insert_template_data(map);
    }
}

pub trait DefaultConfigTrait: Debug + Sized {
    type DetectError: Debug + Display;
    fn detect() -> Result<Self, Self::DetectError>;

    type RequiredConfig: RequiredConfigTrait;
    type UpgradeError: Debug + Display;
    fn upgrade(self) -> Result<Self::RequiredConfig, Self::UpgradeError>;
}

pub trait RequiredConfigTrait: Debug + Serialize + Sized {
    type PromptError: Debug + Display;
    fn prompt(wrapper: &util::TextWrapper) -> Result<Self, Self::PromptError>;
}
