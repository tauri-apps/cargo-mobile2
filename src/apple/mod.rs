pub mod config;
mod deps;
pub mod device;
pub mod ios_deploy;
pub(crate) mod project;
mod system_profile;
pub mod target;
mod teams;
mod xcode_plugin;

pub static NAME: &'static str = "apple";
