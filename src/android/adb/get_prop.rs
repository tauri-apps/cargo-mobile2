use super::adb;
use crate::{
    android::env::Env,
    util::cli::{Report, Reportable},
};
use std::str;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `adb shell getprop {prop}`: {source}")]
    LookupFailed {
        prop: String,
        source: super::RunCheckedError,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    fn prop(&self) -> &str {
        match self {
            Self::LookupFailed { prop, .. } => prop,
            Self::Io(_) => unreachable!(),
        }
    }
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = format!("Failed to run `adb shell getprop {}`", self.prop());
        match self {
            Self::LookupFailed { source, .. } => source.report(&msg),
            Self::Io(err) => Report::error("IO error", err),
        }
    }
}

pub fn get_prop(env: &Env, serial_no: &str, prop: &str) -> Result<String, Error> {
    let prop_ = prop.to_string();
    let handle = adb(env, serial_no)
        .before_spawn(move |cmd| {
            cmd.args(&["shell", "getprop", &prop_]);
            Ok(())
        })
        .stdout_capture()
        .start()?;
    let output = handle.wait()?;
    super::check_authorized(output).map_err(|source| Error::LookupFailed {
        prop: prop.to_owned(),
        source,
    })
}
