#![deny(unsafe_code)]

pub mod android;
#[cfg(target_os = "macos")]
pub mod apple;
pub mod config;
pub mod device;
pub mod dot_cargo;
pub mod env;
pub mod init;
pub mod opts;
pub mod os;
pub mod project;
pub mod target;
pub mod templating;
pub mod update;
pub mod util;

pub static NAME: &'static str = "mobile";
