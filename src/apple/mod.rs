#[cfg(feature = "cli")]
pub mod cli;
pub mod config;
pub mod deps;
pub mod device;
pub mod project;
pub(crate) mod system_profile;
pub mod target;
pub mod teams;
mod version_number;

use std::path::PathBuf;

pub static NAME: &str = "apple";

#[derive(Clone)]
pub struct AuthCredentials {
    pub key_path: PathBuf,
    pub key_id: String,
    pub key_issuer_id: String,
}

pub fn device_ctl_available() -> bool {
    matches!(
        os_info::get().version(),
        os_info::Version::Semantic(major, _, _)
        if *major >= 14)
}
