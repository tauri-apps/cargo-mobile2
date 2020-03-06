use super::adb;
use crate::env::Env;
use ginit_core::exports::into_result::{command::CommandError, IntoResult as _};
use std::{
    fmt::{self, Display},
    str,
};

#[derive(Debug)]
pub enum Error {
    LookupFailed { prop: String, cause: CommandError },
    InvalidUtf8 { prop: String, cause: str::Utf8Error },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LookupFailed { prop, cause } => {
                write!(f, "Failed to run `adb shell getprop {}`: {}", prop, cause)
            }
            Self::InvalidUtf8 { prop, cause } => {
                write!(f, "`{}` contained invalid UTF-8: {}", prop, cause)
            }
        }
    }
}

pub fn get_prop(env: &Env, serial_no: &str, prop: &str) -> Result<String, Error> {
    let output = adb(env, serial_no)
        .args(&["shell", "getprop", prop])
        .output()
        .into_result()
        .map_err(|cause| Error::LookupFailed {
            prop: prop.to_owned(),
            cause,
        })?;
    let raw = str::from_utf8(&output.stdout).map_err(|cause| Error::InvalidUtf8 {
        prop: prop.to_owned(),
        cause,
    })?;
    Ok(raw.trim().to_owned())
}
