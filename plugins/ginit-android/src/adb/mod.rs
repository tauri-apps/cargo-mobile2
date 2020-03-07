pub mod device_list;
pub mod device_name;
pub mod get_prop;

pub use self::{device_list::device_list, device_name::device_name, get_prop::get_prop};

use crate::env::Env;
use ginit_core::{
    exports::into_result::{command::CommandError, IntoResult as _},
    util::PureCommand,
};
use std::{
    fmt::{self, Display},
    process::{Command, Output},
    str,
};

pub fn adb(env: &Env, serial_no: &str) -> Command {
    let mut command = PureCommand::new("adb", env);
    command.args(&["-s", serial_no]);
    command
}

#[derive(Debug)]
pub enum RunCheckedError {
    InvalidUtf8(str::Utf8Error),
    Unauthorized,
    CommandFailed(CommandError),
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

fn run_checked(command: &mut Command) -> Result<Output, RunCheckedError> {
    let result = command.output();
    if let Ok(output) = &result {
        let err = str::from_utf8(&output.stderr).map_err(RunCheckedError::InvalidUtf8)?;
        if err.contains("error: device unauthorized") {
            return Err(RunCheckedError::Unauthorized);
        }
    }
    result.into_result().map_err(RunCheckedError::CommandFailed)
}
