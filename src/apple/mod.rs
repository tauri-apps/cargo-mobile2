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

use crate::util::{
    self,
    cli::{Report, TextWrapper},
};

pub static NAME: &str = "apple";

#[derive(Clone)]
pub struct AuthCredentials {
    pub key_path: PathBuf,
    pub key_id: String,
    pub key_issuer_id: String,
}

pub fn rust_version_check(wrapper: &TextWrapper) -> Result<(), util::RustVersionError> {
    util::RustVersion::check().map(|version| if !version.valid() {
        Report::action_request(
            format!("iOS linking is broken on Rust versions later than 1.45.2 (d3fb005a3 2020-07-31) and earlier than 1.49.0-nightly (ffa2e7ae8 2020-10-24), but you're on {}!", version),
            "This is fixed in Rust 1.49.0 and later:\n`rustup update stable && rustup default stable`",
        ).print(wrapper);
    })
}

pub fn device_ctl_available() -> bool {
    matches!(
        os_info::get().version(),
        os_info::Version::Semantic(major, _, _)
        if *major >= 14)
}
