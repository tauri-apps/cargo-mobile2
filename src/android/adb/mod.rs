pub mod device_list;
pub mod device_name;
pub mod get_prop;

pub use self::{device_list::device_list, device_name::device_name, get_prop::get_prop};

use super::env::Env;
use crate::{env::ExplicitEnv as _, util::cli::Report};
use std::{
    fmt::{self, Display},
    str,
};

pub fn adb(env: &Env, serial_no: &str) -> bossy::Command {
    bossy::Command::pure("adb")
        .with_env_vars(env.explicit_env())
        .with_args(&["-s", serial_no])
}

#[derive(Debug)]
pub enum RunCheckedError {
    InvalidUtf8(str::Utf8Error),
    Unauthorized,
    CommandFailed(bossy::Error),
}

impl Display for RunCheckedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8(err) => {
                write!(f, "stderr contained invalid UTF-8: {}", err)
            }
            Self::Unauthorized => write!(f, "This device doesn't yet trust this computer. On the device, you should see a prompt like \"Allow USB debugging?\". Pressing \"Allow\" should fix this."),
            Self::CommandFailed(err) => write!(f, "Failed to run adb command: {}", err),
        }
    }
}

impl RunCheckedError {
    pub fn report(&self, msg: impl Display) -> Report {
        match self {
            Self::Unauthorized => Report::action_request(msg, self),
            _ => Report::error(msg, self),
        }
    }
}

fn run_checked(command: &mut bossy::Command) -> Result<bossy::Output, RunCheckedError> {
    let result = command.run_and_wait_for_output();
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
