pub mod device_list;
pub mod device_name;
pub mod get_prop;

pub use self::{device_list::device_list, device_name::device_name, get_prop::get_prop};

use super::env::Env;
use crate::{env::ExplicitEnv as _, util::cli::Report, DuctExpressionExt};
use std::{ffi::OsString, str, string::FromUtf8Error};
use thiserror::Error;

pub fn adb<U>(env: &Env, args: U) -> duct::Expression
where
    U: IntoIterator,
    U::Item: Into<OsString>,
{
    duct::cmd(env.platform_tools_path().join("adb"), args).vars(env.explicit_env())
}

#[derive(Debug, Error)]
pub enum RunCheckedError {
    #[error(transparent)]
    InvalidUtf8(#[from] FromUtf8Error),
    #[error("This device doesn't yet trust this computer. On the device, you should see a prompt like \"Allow USB debugging?\". Pressing \"Allow\" should fix this.")]
    Unauthorized,
    #[error(transparent)]
    CommandFailed(std::io::Error),
}

impl RunCheckedError {
    pub fn report(&self, msg: &str) -> Report {
        match self {
            Self::InvalidUtf8(err) => Report::error(msg, err),
            Self::Unauthorized => Report::action_request(msg, self),
            Self::CommandFailed(err) => Report::error(msg, err),
        }
    }
}

fn check_authorized(output: &std::process::Output) -> Result<String, RunCheckedError> {
    if !output.status.success() {
        if let Ok(stderr) = String::from_utf8(output.stderr.clone()) {
            if stderr.contains("error: device unauthorized") {
                return Err(RunCheckedError::Unauthorized);
            }
        }
    }
    let stdout = String::from_utf8(output.stdout.clone())?.trim().to_string();
    Ok(stdout)
}
