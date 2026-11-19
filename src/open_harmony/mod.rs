#[cfg(feature = "cli")]
pub mod cli;
pub mod config;
pub mod device;
pub mod emulator;
pub mod env;
pub mod hap;
pub mod hdc;
pub mod ohpm;
pub(crate) mod project;
pub mod target;

pub static NAME: &str = "open-harmony";
