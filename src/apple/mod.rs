pub mod cli;
pub(crate) mod config;
mod deps;
mod device;
mod ios_deploy;
pub(crate) mod project;
mod system_profile;
mod target;
mod teams;

use crate::util::{
    self,
    cli::{Report, TextWrapper},
};

pub static NAME: &str = "apple";

pub fn rust_version_check(wrapper: &TextWrapper) -> Result<(), util::RustVersionError> {
    util::RustVersion::check().map(|version| if !version.valid() {
        Report::action_request(
            format!("iOS linking is broken on Rust versions later than 1.45.2 (d3fb005a3 2020-07-31) and earlier than 1.49.0-nightly (ffa2e7ae8 2020-10-24), but you're on {}!", version),
            "Until this is resolved by Rust 1.49.0, please do one of the following:\nA) downgrade to 1.45.2:\n    `rustup install stable-2020-08-03 && rustup default stable-2020-08-03`\nB) update to a recent nightly:\n    `rustup update nightly && rustup default nightly`",
        ).print(wrapper);
    })
}
