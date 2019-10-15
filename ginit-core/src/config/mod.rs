pub mod app_name;
pub mod empty;
pub mod shared;
pub mod umbrella;

use crate::{templating, util};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

pub trait ConfigTrait: Debug + Serialize + Sized {
    type DefaultConfig: DefaultConfigTrait;

    type Raw: for<'r> Deserialize<'r> + Serialize;
    type Error: Debug + Display;
    fn from_raw(shared: shared::Shared, raw: Option<Self::Raw>) -> Result<Self, Self::Error>;

    fn shared(&self) -> &shared::Shared;

    fn init_templating(&self) -> bicycle::Bicycle {
        templating::init(Some(self.shared()))
    }

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
