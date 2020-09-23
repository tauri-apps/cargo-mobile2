pub mod adb;
pub mod config;
pub mod device;
pub mod env;
mod ndk;
pub(crate) mod project;
pub mod target;

pub static NAME: &'static str = "android";
