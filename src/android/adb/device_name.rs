use super::adb;
use crate::{
    android::env::Env,
    util::{Report, Reportable},
};
use once_cell_regex::regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `adb emu avd name`: {0}")]
    EmuFailed(#[source] super::RunCheckedError),
    #[error("Failed to run `adb shell dumpsys bluetooth_manager`: {0}")]
    DumpsysFailed(#[source] super::RunCheckedError),
    #[error("Name regex didn't match anything.")]
    NotMatched,
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = "Failed to get device name";
        match self {
            Self::EmuFailed(err) => err.report("Failed to run `adb emu avd name`"),
            Self::DumpsysFailed(err) => {
                err.report("Failed to run `adb shell dumpsys bluetooth_manager`")
            }
            Self::NotMatched => Report::error(msg, self),
        }
    }
}

pub fn device_name(env: &Env, serial_no: &str) -> Result<String, Error> {
    if serial_no.starts_with("emulator") {
        super::check_authorized(
            adb(env, serial_no)
                .with_args(&["emu", "avd", "name"])
                .run_and_wait_for_str(|raw| Ok(raw.split('\n').next().unwrap().trim().into())),
        )
        .map_err(Error::EmuFailed)?
    } else {
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
}
