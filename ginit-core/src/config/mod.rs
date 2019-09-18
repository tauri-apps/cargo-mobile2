pub mod app_name;
mod core;
mod shared;

pub use self::{core::*, shared::*};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

// this will be renamed to `Config` once root config moves to `ginit`
pub trait ConfigTrait: Debug + Serialize + Sized {
    type Raw: for<'r> Deserialize<'r> + Serialize;
    type Error: Debug + Display;

    fn from_raw(shared: Rc<SharedConfig>, raw: Self::Raw) -> Result<Self, Self::Error>;

    fn shared(&self) -> &SharedConfig;

    fn insert_template_data(&self, key: &str, map: &mut bicycle::JsonMap) {
        map.insert(key, self);
        self.shared().insert_template_data(map);
    }
}
