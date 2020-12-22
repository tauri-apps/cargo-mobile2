pub mod device_list;
pub mod device_name;
pub mod get_prop;

pub use self::{device_list::device_list, device_name::device_name, get_prop::get_prop};

use super::env::Env;
use crate::{env::ExplicitEnv as _, util::cli::Report};
use std::str;

pub fn adb(env: &Env, serial_no: &str) -> bossy::Command {
    bossy::Command::pure("adb")
        .with_env_vars(env.explicit_env())
        .with_args(&["-s", serial_no])
}

#[derive(Debug)]
pub enum RunCheckedError {
    InvalidUtf8(bossy::Error),
    Unauthorized,
    CommandFailed(bossy::Error),
}

impl RunCheckedError {
    pub fn report(&self, msg: &str) -> Report {
        match self {
            Self::InvalidUtf8(err) => Report::error(msg, err),
            Self::Unauthorized => Report::action_request(msg, "This device doesn't yet trust this computer. On the device, you should see a prompt like \"Allow USB debugging?\". Pressing \"Allow\" should fix this."),
            Self::CommandFailed(err) => Report::error(msg, err),
        }
    }
}

fn check_authorized<T>(result: bossy::Result<T>) -> Result<T, RunCheckedError> {
    if let Err(err) = &result {
        if let Some(stderr) = err
            .stderr_str()
            .transpose()
            .map_err(RunCheckedError::InvalidUtf8)?
        {
            if stderr.contains("error: device unauthorized") {
                return Err(RunCheckedError::Unauthorized);
            }
        }
    }
    result.map_err(RunCheckedError::CommandFailed)
}
