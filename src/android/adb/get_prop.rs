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
}

impl Error {
    fn prop(&self) -> &str {
        match self {
            Self::LookupFailed { prop, .. } => prop,
        }
    }
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = format!("Failed to run `adb shell getprop {}`", self.prop());
        match self {
            Self::LookupFailed { source, .. } => source.report(&msg),
        }
    }
}

pub fn get_prop(env: &Env, serial_no: &str, prop: &str) -> Result<String, Error> {
    super::check_authorized(
        adb(env, serial_no)
            .with_args(&["shell", "getprop", prop])
            .run_and_wait_for_str(|s| s.trim().to_owned()),
    )
    .map_err(|source| Error::LookupFailed {
        prop: prop.to_owned(),
        source,
    })
}
