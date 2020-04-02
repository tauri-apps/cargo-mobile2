#![deny(unsafe_code)]

pub mod android;
pub mod apple;
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
