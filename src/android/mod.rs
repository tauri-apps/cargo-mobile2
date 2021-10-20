pub(crate) mod adb;
mod bundletool;
pub mod cli;
pub(crate) mod config;
mod device;
pub(crate) mod env;
mod jnilibs;
mod ndk;
pub(crate) mod project;
mod source_props;
mod target;

pub static NAME: &str = "android";
