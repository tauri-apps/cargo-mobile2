use super::{device_name, get_prop};
use crate::{device::Device, env::Env, target::Target};
use ginit_core::{exports::once_cell_regex::regex_multi_line, util::PureCommand};
use std::{
    collections::BTreeSet,
    fmt::{self, Display},
    str,
};

#[derive(Debug)]
pub enum Error {
    DevicesFailed(super::RunCheckedError),
    InvalidUtf8(str::Utf8Error),
    NameFailed(device_name::Error),
    ModelFailed(get_prop::Error),
    AbiFailed(get_prop::Error),
    AbiInvalid(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DevicesFailed(err) => write!(f, "Failed to run `adb devices`: {}", err),
            Self::InvalidUtf8(err) => write!(f, "Device list contained invalid UTF-8: {}", err),
            Self::NameFailed(err) => write!(f, "Failed to get device name: {}", err),
            Self::ModelFailed(err) => write!(f, "Failed to get device model: {}", err),
            Self::AbiFailed(err) => write!(f, "Failed to get device ABI: {}", err),
            Self::AbiInvalid(abi) => write!(f, "{:?} isn't a valid target ABI.", abi),
        }
    }
}

pub fn device_list(env: &Env) -> Result<BTreeSet<Device<'static>>, Error> {
    let serial_re = regex_multi_line!(r"^([\w\d]{6,20})	\b");
    let output = super::run_checked(PureCommand::new("adb", env).args(&["devices"]))
        .map_err(Error::DevicesFailed)?;
    let raw_list = str::from_utf8(&output.stdout).map_err(Error::InvalidUtf8)?;
    serial_re
        .captures_iter(raw_list)
        .map(|caps| {
            assert_eq!(caps.len(), 2);
            let serial_no = caps.get(1).unwrap().as_str().to_owned();
            let name = device_name(env, &serial_no).map_err(Error::NameFailed)?;
            let model =
                get_prop(env, &serial_no, "ro.product.model").map_err(Error::ModelFailed)?;
            let abi = get_prop(env, &serial_no, "ro.product.cpu.abi").map_err(Error::AbiFailed)?;
            let target = Target::for_abi(&abi).ok_or_else(|| Error::AbiInvalid(abi.clone()))?;
            Ok(Device::new(serial_no, name, model, target))
        })
        .collect()
}
