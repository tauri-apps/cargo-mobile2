pub mod app_name;
pub mod empty;
pub mod gen;
pub mod shared;
pub mod umbrella;

use crate::{templating, util};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

pub trait ConfigTrait: Debug + Serialize + Sized {
    type Raw: RawConfigTrait;
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

pub trait RawConfigTrait: Debug + for<'r> Deserialize<'r> + Serialize {
    fn is_zst() -> bool {
        std::mem::size_of::<Self>() == 0
    }

    type Detected: DetectedConfigTrait;

    type FromDetectedError: Debug + Display;
    fn from_detected(detected: Self::Detected) -> Result<Self, Self::FromDetectedError>;

    type FromPromptError: Debug + Display;
    fn from_prompt(
        detected: Self::Detected,
        wrapper: &util::TextWrapper,
    ) -> Result<Self, Self::FromPromptError>;
}

pub trait DetectedConfigTrait: Debug + Sized {
    type Error: Debug + Display;
    fn new() -> Result<Self, Self::Error>;
}
