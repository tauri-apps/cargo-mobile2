mod aidl;
pub mod parcel;
mod services;

use self::{aidl::Aidl, parcel::Parcel, services::SERVICES};
use super::{adb, get_prop};
use crate::env::Env;
use ginit_core::exports::into_result::{command::CommandError, IntoResult as _};
use std::{
    fmt::{self, Display},
    str,
};

#[derive(Debug)]
pub enum Error {
    ServiceUnknown(String),
    VersionFailed(get_prop::Error),
    AidlFailed(aidl::Error),
    FunctionUnknown { service: String, function: String },
    CommandFailed(CommandError),
    InvalidUtf8(str::Utf8Error),
    ParcelInvalid(parcel::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ServiceUnknown(service) => write!(f, "Service {:?} unknown.", service),
            Self::VersionFailed(err) => {
                write!(f, "Failed to get device's Android version: {}", err)
            }
            Self::AidlFailed(err) => write!(f, "Failed to load or fetch AIDL: {}", err),
            Self::FunctionUnknown { service, function } => write!(
                f,
                "Function {:?} unknown for service {:?}.",
                function, service
            ),
            Self::CommandFailed(err) => write!(f, "Failed to run service call: {}", err),
            Self::InvalidUtf8(err) => write!(f, "Output contained invalid UTF-8: {}", err),
            Self::ParcelInvalid(err) => write!(f, "Output wasn't a valid parcel: {}", err),
        }
    }
}

pub fn service_call(
    env: &Env,
    serial_no: &str,
    service: &str,
    function: &str,
) -> Result<Parcel, Error> {
    if SERVICES.contains(&service) {
        let version =
            get_prop(env, serial_no, "ro.build.version.release").map_err(Error::VersionFailed)?;
        let aidl = Aidl::load_or_fetch(&version, service).map_err(Error::AidlFailed)?;
        let index = aidl.index(function).ok_or_else(|| Error::FunctionUnknown {
            service: service.to_owned(),
            function: function.to_owned(),
        })?;
        let output = adb(env, serial_no)
            .args(&["shell", "service", "call", service])
            .arg(index.to_string())
            .output()
            .into_result()
            .map_err(Error::CommandFailed)?;
        let output = str::from_utf8(&output.stdout).map_err(Error::InvalidUtf8)?;
        Parcel::from_output(&output).map_err(Error::ParcelInvalid)
    } else {
        Err(Error::ServiceUnknown(service.to_owned()))
    }
}
