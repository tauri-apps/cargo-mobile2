use crate::{
    android::env::Env,
    util::cli::{Report, Reportable},
};
use std::{str, time::Duration};
use thiserror::Error;

use super::adb;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `adb shell getprop {prop}`: {source}")]
    LookupFailed {
        prop: String,
        source: super::RunCheckedError,
    },
    #[error("Failed to run {command}: {error}")]
    CommandFailed {
        command: String,
        error: std::io::Error,
    },
}

impl Error {
    fn prop(&self) -> &str {
        match self {
            Self::LookupFailed { prop, .. } => prop,
            Self::CommandFailed { .. } => unreachable!(),
        }
    }
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = format!("Failed to run `adb shell getprop {}`", self.prop());
        match self {
            Self::LookupFailed { source, .. } => source.report(&msg),
            Self::CommandFailed { command, error } => {
                Report::error(format!("Failed to run {command}"), error)
            }
        }
    }
}

pub fn get_prop(env: &Env, serial_no: &str, prop: &str) -> Result<String, Error> {
    let cmd = adb(env, ["-s", serial_no, "shell", "getprop", prop]);
    let handle = cmd
        .stdin_file(os_pipe::dup_stdin().unwrap())
        .stdout_capture()
        .stderr_capture()
        .start()
        .map_err(|error| Error::CommandFailed {
            command: format!("{cmd:?}"),
            error,
        })?;

    let output = handle
        .wait_timeout(Duration::from_secs(3))
        .and_then(|output| {
            output.ok_or(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "`adb shell getprop` timed out",
            ))
        })
        .map_err(|error| Error::CommandFailed {
            command: format!("{cmd:?}"),
            error,
        })?;
    super::check_authorized(output).map_err(|source| Error::LookupFailed {
        prop: prop.to_owned(),
        source,
    })
}
