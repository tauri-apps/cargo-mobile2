use super::adb;
use crate::{
    android::env::Env,
    util::cli::{Report, Reportable},
};
use std::str;

#[derive(Debug)]
pub enum Error {
    LookupFailed {
        prop: String,
        cause: super::RunCheckedError,
    },
    InvalidUtf8 {
        prop: String,
        cause: str::Utf8Error,
    },
}

impl Error {
    fn prop(&self) -> &str {
        match self {
            Self::LookupFailed { prop, .. } | Self::InvalidUtf8 { prop, .. } => prop,
        }
    }
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = format!("Failed to run `adb shell getprop {}`", self.prop());
        match self {
            Self::LookupFailed { cause, .. } => cause.report(&msg),
            Self::InvalidUtf8 { cause, .. } => {
                Report::error(msg, format!("Output contained invalid UTF-8: {}", cause))
            }
        }
    }
}

pub fn get_prop(env: &Env, serial_no: &str, prop: &str) -> Result<String, Error> {
    let output =
        super::run_checked(&mut adb(env, serial_no).with_args(&["shell", "getprop", prop]))
            .map_err(|cause| Error::LookupFailed {
                prop: prop.to_owned(),
                cause,
            })?;
    output
        .stdout_str()
        .map_err(|cause| Error::InvalidUtf8 {
            prop: prop.to_owned(),
            cause,
        })
        .map(|raw| raw.trim().to_owned())
}
