mod section;

use crate::{
    env,
    os::Env,
    util::{self, cli::TextWrapper},
};
use thiserror::Error;

// This should only be used for errors that we *really* don't expect and/or
// that violate core assumptions made throughout the program.
#[derive(Debug, Error)]
pub enum Unrecoverable {
    // Only encountered if the most basic environment variables are absent or
    // unreadable
    #[error(transparent)]
    EnvInitFailed(#[from] env::Error),
    // Only encountered if A) the user has no home directory, or B) either the
    // home or some other path isn't valid UTF-8
    #[error("Failed to prettify path: {0}")]
    ContractHomeFailed(#[from] util::ContractHomeError),
}

pub fn exec(wrapper: &TextWrapper) -> Result<(), Unrecoverable> {
    let env = Env::new()?;
    section::cargo_mobile::check()?.print(wrapper);
    #[cfg(target_os = "macos")]
    section::apple::check().print(wrapper);
    section::android::check(&env)?.print(wrapper);
    section::device_list::check(&env).print(wrapper);
    Ok(())
}
