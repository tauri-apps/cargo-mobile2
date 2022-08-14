pub mod cli;
pub(crate) mod config;
pub(crate) mod deps;
mod device;
pub(crate) mod ios_deploy;
pub(crate) mod project;
pub(crate) mod system_profile;
mod target;
pub(crate) mod teams;
mod version_number;

use crate::util::{
    self,
    cli::{Report, TextWrapper},
};

pub static NAME: &str = "apple";

pub fn rust_version_check(wrapper: &TextWrapper) -> Result<(), util::RustVersionError> {
    util::RustVersion::check().map(|version| if !version.valid() {
        Report::action_request(
            format!("iOS linking is broken on Rust versions later than 1.45.2 (d3fb005a3 2020-07-31) and earlier than 1.49.0-nightly (ffa2e7ae8 2020-10-24), but you're on {}!", version),
            "This is fixed in Rust 1.49.0 and later:\n`rustup update stable && rustup default stable`",
        ).print(wrapper);
    })
}
