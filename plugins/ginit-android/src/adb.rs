use crate::{device::Device, env::Env, target::Target};
use ginit_core::{regex, regex_multi_line, util::pure_command::PureCommand};
use into_result::{command::CommandError, IntoResult as _};
use std::{collections::BTreeSet, fmt, process::Command, str};

pub fn adb(env: &Env, serial_no: &str) -> Command {
    let mut command = PureCommand::new("adb", env);
    command.args(&["-s", serial_no]);
    command
}

#[derive(Debug)]
pub enum GetPropError {
    LookupFailed { prop: String, cause: CommandError },
    InvalidUtf8 { prop: String, cause: str::Utf8Error },
}

impl fmt::Display for GetPropError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GetPropError::LookupFailed { prop, cause } => {
                write!(f, "Failed to run `adb shell getprop {}`: {}", prop, cause)
            }
            GetPropError::InvalidUtf8 { prop, cause } => {
                write!(f, "`{}` contained invalid UTF-8: {}", prop, cause)
            }
        }
    }
}

pub fn get_prop<T>(
    env: &Env,
    serial_no: &str,
    prop: &str,
    f: impl FnOnce(&str) -> T,
) -> Result<T, GetPropError> {
    let output = adb(env, serial_no)
        .args(&["shell", "getprop", prop])
        .output()
        .into_result()
        .map_err(|cause| GetPropError::LookupFailed {
            prop: prop.to_owned(),
            cause,
        })?;
    let raw = str::from_utf8(&output.stdout).map_err(|cause| GetPropError::InvalidUtf8 {
        prop: prop.to_owned(),
        cause,
    })?;
    Ok(f(raw.trim()))
}

#[derive(Debug)]
pub enum DeviceNameError {
    DumpsysFailed(CommandError),
    InvalidUtf8(str::Utf8Error),
    NotMatched,
}

impl fmt::Display for DeviceNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceNameError::DumpsysFailed(err) => write!(
                f,
                "Failed to run `adb shell dumpsys bluetooth_manager`: {}",
                err
            ),
            DeviceNameError::InvalidUtf8(err) => {
                write!(f, "Bluetooth info contained invalid UTF-8: {}", err)
            }
            DeviceNameError::NotMatched => write!(f, "Name regex didn't match anything."),
        }
    }
}

pub fn device_name(env: &Env, serial_no: &str) -> Result<String, DeviceNameError> {
    let name_re = regex!(r#"\bname: (.*)"#);
    let output = adb(env, serial_no)
        .args(&["shell", "dumpsys", "bluetooth_manager"])
        .output()
        .into_result()
        .map_err(DeviceNameError::DumpsysFailed)?;
    let raw = str::from_utf8(&output.stdout).map_err(DeviceNameError::InvalidUtf8)?;
    name_re
        .captures_iter(raw)
        .next()
        .map(|caps| {
            assert_eq!(caps.len(), 2);
            caps.get(1).unwrap().as_str().to_owned()
        })
        .ok_or_else(|| DeviceNameError::NotMatched)
}

#[derive(Debug)]
pub enum DeviceListError {
    DevicesFailed(CommandError),
    InvalidUtf8(str::Utf8Error),
    NameFailed(DeviceNameError),
    ModelFailed(GetPropError),
    AbiFailed(GetPropError),
    AbiInvalid(String),
}

impl fmt::Display for DeviceListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceListError::DevicesFailed(err) => {
                write!(f, "Failed to run `adb devices`: {}", err)
            }
            DeviceListError::InvalidUtf8(err) => {
                write!(f, "Device list contained invalid UTF-8: {}", err)
            }
            DeviceListError::NameFailed(err) => write!(f, "Failed to get device name: {}", err),
            DeviceListError::ModelFailed(err) => write!(f, "Failed to get device model: {}", err),
            DeviceListError::AbiFailed(err) => write!(f, "Failed to get device ABI: {}", err),
            DeviceListError::AbiInvalid(abi) => write!(f, "{:?} isn't a valid target ABI.", abi),
        }
    }
}

pub fn device_list(env: &Env) -> Result<BTreeSet<Device<'static>>, DeviceListError> {
    let serial_re = regex_multi_line!(r#"^([\w\d]{6,20})	\b"#);
    let output = PureCommand::new("adb", env)
        .args(&["devices"])
        .output()
        .into_result()
        .map_err(DeviceListError::DevicesFailed)?;
    let raw_list = str::from_utf8(&output.stdout).map_err(DeviceListError::InvalidUtf8)?;
    serial_re
        .captures_iter(raw_list)
        .map(|caps| {
            assert_eq!(caps.len(), 2);
            let serial_no = caps.get(1).unwrap().as_str().to_owned();
            let name = device_name(env, &serial_no).map_err(DeviceListError::NameFailed)?;
            let model = get_prop(env, &serial_no, "ro.product.model", |model| {
                model.to_owned()
            })
            .map_err(DeviceListError::ModelFailed)?;
            let abi = get_prop(env, &serial_no, "ro.product.cpu.abi", |abi| abi.to_owned())
                .map_err(DeviceListError::AbiFailed)?;
            let target =
                Target::for_abi(&abi).ok_or_else(|| DeviceListError::AbiInvalid(abi.clone()))?;
            Ok(Device::new(serial_no, name, model, target))
        })
        .collect()
}
