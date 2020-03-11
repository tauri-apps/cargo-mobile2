#![forbid(unsafe_code)]

pub mod cargo;
pub mod config;
pub mod device;
pub mod env;
pub mod opts;
pub mod storage;
pub mod target;
pub mod templating;
pub mod util;

pub mod exports {
    pub use bicycle;
    pub use colored;
    pub use dirs;
    pub use ginit_os::exports::bossy;
    pub use once_cell_regex::{
        self,
        exports::{once_cell, regex},
    };
    pub use toml;
}

pub use ginit_os as os;

pub static NAME: &'static str = "ginit";
