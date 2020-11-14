mod adb;
pub mod cli;
pub(crate) mod config;
mod device;
pub(crate) mod env;
mod jnilibs;
mod ndk;
pub(crate) mod project;
mod target;

pub static NAME: &str = "android";
