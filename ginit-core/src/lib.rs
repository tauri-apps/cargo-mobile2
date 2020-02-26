#![forbid(unsafe_code)]

pub mod bundle;
pub mod cargo;
pub mod config;
pub mod device;
pub mod env;
pub mod opts;
pub mod target;
pub mod templating;
pub mod util;

pub mod exports {
    pub use bicycle;
    pub use colored;
    pub use dirs;
    pub use ginit_os::exports::into_result;
    pub use once_cell_regex::{
        self,
        exports::{once_cell, regex},
    };
    pub use toml;
}

pub use ginit_os as os;

pub static NAME: &'static str = "ginit";
