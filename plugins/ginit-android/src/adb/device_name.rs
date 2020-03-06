use super::adb;
use crate::env::Env;
use ginit_core::exports::{
    into_result::{command::CommandError, IntoResult as _},
    once_cell_regex::regex,
};
use std::{
    fmt::{self, Display},
    str,
};

#[derive(Debug)]
pub enum Error {
    DumpsysFailed(CommandError),
    InvalidUtf8(str::Utf8Error),
    NotMatched,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DumpsysFailed(err) => write!(
                f,
                "Failed to run `adb shell dumpsys bluetooth_manager`: {}",
                err
            ),
            Self::InvalidUtf8(err) => write!(f, "Bluetooth info contained invalid UTF-8: {}", err),
            Self::NotMatched => write!(f, "Name regex didn't match anything."),
        }
    }
}

pub fn device_name(env: &Env, serial_no: &str) -> Result<String, Error> {
    let name_re = regex!(r"\bname: (?P<name>.*)");
    let output = adb(env, serial_no)
        .args(&["shell", "dumpsys", "bluetooth_manager"])
        .output()
        .into_result()
        .map_err(Error::DumpsysFailed)?;
    let raw = str::from_utf8(&output.stdout).map_err(Error::InvalidUtf8)?;
    name_re
        .captures(raw)
        .map(|caps| caps["name"].to_owned())
        .ok_or_else(|| Error::NotMatched)
}
