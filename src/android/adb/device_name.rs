use super::adb;
use crate::{
    android::env::Env,
    util::cli::{Report, Reportable},
};
use once_cell_regex::regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `adb shell dumpsys bluetooth_manager`: {0}")]
    DumpsysFailed(#[source] super::RunCheckedError),
    #[error("Name regex didn't match anything.")]
    NotMatched,
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = "Failed to get device name";
        match self {
            Self::DumpsysFailed(err) => {
                err.report("Failed to run `adb shell dumpsys bluetooth_manager`")
            }
            Self::NotMatched => Report::error(msg, self),
        }
    }
}

pub fn device_name(env: &Env, serial_no: &str) -> Result<String, Error> {
    super::check_authorized(
        adb(env, serial_no)
            .with_args(&["shell", "dumpsys", "bluetooth_manager"])
            .run_and_wait_for_str(|raw| {
                regex!(r"\bname: (?P<name>.*)")
                    .captures(raw)
                    .map(|caps| caps["name"].to_owned())
                    .ok_or_else(|| Error::NotMatched)
            }),
    )
    .map_err(Error::DumpsysFailed)?
}
