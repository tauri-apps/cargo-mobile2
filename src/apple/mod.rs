pub mod config;
mod deps;
pub mod device;
pub mod ios_deploy;
pub(crate) mod project;
mod system_profile;
pub mod target;
mod teams;

use crate::util::{
    self,
    cli::{Report, TextWrapper},
};

pub static NAME: &'static str = "apple";

// This will be removed when the upstream regression is fixed...
pub fn rust_version_check(wrapper: &TextWrapper) -> Result<(), util::RustVersionError> {
    util::RustVersion::check().map(|version| {
        const MAX: (u32, u32, u32) = (1, 45, 2);
        if version.triple > MAX {
            Report::action_request(
                format!("iOS linking is currently broken on Rust versions later than 1.45.2, and you're on {}!", version),
                "Until this is resolved upstream, switch back to 1.45.2 by running `rustup install stable-2020-08-03 && rustup default stable-2020-08-03`",
            ).print(wrapper);
        }
    })
}
