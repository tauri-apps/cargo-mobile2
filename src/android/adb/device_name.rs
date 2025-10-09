use super::adb;
use crate::{
    android::env::Env,
    util::cli::{Report, Reportable},
};
use once_cell_regex::regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `adb emu avd name`: {0}")]
    EmuFailed(#[source] super::RunCheckedError),
    #[error(transparent)]
    GetPropFailed(super::get_prop::Error),
    #[error("Failed to run `adb shell dumpsys bluetooth_manager`: {0}")]
    DumpsysFailed(#[source] super::RunCheckedError),
    #[error("Name regex didn't match anything.")]
    NotMatched,
    #[error("Failed to run {command}: {error}")]
    CommandFailed {
        command: String,
        error: std::io::Error,
    },
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = "Failed to get device name";
        match self {
            Self::EmuFailed(err) => err.report("Failed to run `adb emu avd name`"),
            Self::GetPropFailed(err) => err.report(),
            Self::DumpsysFailed(err) => {
                err.report("Failed to run `adb shell dumpsys bluetooth_manager`")
            }
            Self::NotMatched => Report::error(msg, self),
            Self::CommandFailed { command, error } => {
                Report::error(format!("Failed to run {command}"), error)
            }
        }
    }
}

pub fn device_name(env: &Env, serial_no: &str) -> Result<String, Error> {
    if serial_no.starts_with("emulator") {
        let cmd = adb(env, ["-s", serial_no, "emu", "avd", "name"])
            .stderr_capture()
            .stdout_capture();
        let name = super::check_authorized(
            cmd.start()
                .map_err(|error| Error::CommandFailed {
                    command: format!("{cmd:?}"),
                    error,
                })?
                .wait()
                .map_err(|error| Error::CommandFailed {
                    command: format!("{cmd:?}"),
                    error,
                })?,
        )
        .map(|stdout| stdout.split('\n').next().unwrap().trim().to_string())
        .map_err(Error::EmuFailed)?;
        if name.is_empty() {
            super::get_prop::get_prop(env, serial_no, "ro.boot.qemu.avd_name")
                .map_err(Error::GetPropFailed)
        } else {
            Ok(name)
        }
    } else {
        let cmd = adb(
            env,
            ["-s", serial_no, "shell", "dumpsys", "bluetooth_manager"],
        )
        .stderr_capture()
        .stdout_capture();
        super::check_authorized(
            cmd.start()
                .map_err(|error| Error::CommandFailed {
                    command: format!("{cmd:?}"),
                    error,
                })?
                .wait()
                .map_err(|error| Error::CommandFailed {
                    command: format!("{cmd:?}"),
                    error,
                })?,
        )
        .map_err(Error::DumpsysFailed)
        .and_then(|stdout| {
            regex!(r"\bname: (?P<name>.*)")
                .captures(&stdout)
                .map(|caps| caps["name"].to_owned())
                .ok_or(Error::NotMatched)
        })
    }
}
