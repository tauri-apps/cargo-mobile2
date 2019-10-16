#![forbid(unsafe_code)]

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
    pub use clap;
    pub use colored;
    pub use into_result;
}

pub static NAME: &'static str = "ginit";
