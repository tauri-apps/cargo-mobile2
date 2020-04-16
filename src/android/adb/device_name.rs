use super::adb;
use crate::{
    android::env::Env,
    util::cli::{Report, Reportable},
};
use once_cell_regex::regex;
use std::str;

#[derive(Debug)]
pub enum Error {
    DumpsysFailed(super::RunCheckedError),
    InvalidUtf8(str::Utf8Error),
    NotMatched,
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = "Failed to get device name";
        match self {
            Self::DumpsysFailed(err) => {
                err.report("Failed to run `adb shell dumpsys bluetooth_manager`")
            }
            Self::InvalidUtf8(err) => {
                Report::error(msg, format!("Output contained invalid UTF-8: {}", err))
            }
            Self::NotMatched => Report::error(msg, "Name regex didn't match anything"),
        }
    }
}

pub fn device_name(env: &Env, serial_no: &str) -> Result<String, Error> {
    let name_re = regex!(r"\bname: (?P<name>.*)");
    let output = super::run_checked(&mut adb(env, serial_no).with_args(&[
        "shell",
        "dumpsys",
        "bluetooth_manager",
    ]))
    .map_err(Error::DumpsysFailed)?;
    let raw = output.stdout_str().map_err(Error::InvalidUtf8)?;
    name_re
        .captures(raw)
        .map(|caps| caps["name"].to_owned())
        .ok_or_else(|| Error::NotMatched)
}
