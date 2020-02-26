mod ffi;
mod open;

pub use self::open::*;

use std::path::PathBuf;

pub fn add_ext_to_bin_name(name: impl AsRef<str>) -> PathBuf {
    name.as_ref().into()
}
