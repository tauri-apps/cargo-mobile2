#![deny(unsafe_code)]

pub mod android;
pub mod apple;
pub mod cli;
pub mod config;
pub mod device;
pub mod dot_cargo;
pub mod env;
pub mod init;
pub mod opts;
pub mod os;
pub mod project;
pub mod steps;
pub mod target;
pub mod templating;
pub mod util;

pub static NAME: &'static str = "mobile";
pub static PKG_NAME: &'static str = env!("CARGO_PKG_NAME");
pub static PKG_NAME_SNAKE: &'static str = "cargo_mobile";
