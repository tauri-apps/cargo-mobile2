pub mod app_name;
mod core;

pub use self::core::*;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

pub trait ConfigTrait: Debug + Serialize + Sized {
    type Raw: for<'r> Deserialize<'r> + Serialize;
    type Error: Debug + Display;

    fn from_raw(shared: Rc<Config>, raw: Self::Raw) -> Result<Self, Self::Error>;

    fn shared(&self) -> &Config;

    fn insert_template_data(&self, key: &str, map: &mut bicycle::JsonMap) {
        map.insert(key, self);
        self.shared().insert_template_data(map);
    }
}
