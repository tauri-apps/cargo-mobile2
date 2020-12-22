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
            Self::LookupFailed { cause, .. } => cause.report(&msg),
        }
    }
}

pub fn get_prop(env: &Env, serial_no: &str, prop: &str) -> Result<String, Error> {
    super::check_authorized(
        adb(env, serial_no)
            .with_args(&["shell", "getprop", prop])
            .run_and_wait_for_str(|s| s.trim().to_owned()),
    )
    .map_err(|cause| Error::LookupFailed {
        prop: prop.to_owned(),
        cause,
    })
}
