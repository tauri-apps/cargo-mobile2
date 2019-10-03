pub mod app_name;
mod shared;

pub use self::shared::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

pub trait ConfigTrait: Debug + Serialize + Sized {
    type Raw: for<'r> Deserialize<'r> + Serialize;
    type Error: Debug + Display;

    fn from_raw(shared: Config, raw: Option<Self::Raw>) -> Result<Self, Self::Error>;

    fn shared(&self) -> &Config;

    fn insert_template_data(&self, key: &str, map: &mut bicycle::JsonMap) {
        map.insert(key, self);
        self.shared().insert_template_data(map);
    }
}
