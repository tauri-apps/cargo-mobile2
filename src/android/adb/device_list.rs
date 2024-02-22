use super::{device_name, get_prop};
use crate::{
    android::{device::Device, env::Env, target::Target},
    env::ExplicitEnv as _,
    util::cli::{Report, Reportable},
};
use once_cell_regex::regex_multi_line;
use std::{collections::BTreeSet, process::Command};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to run `adb devices`: {0}")]
    DevicesFailed(#[from] super::RunCheckedError),
    #[error(transparent)]
    NameFailed(#[from] device_name::Error),
    #[error(transparent)]
    ModelFailed(get_prop::Error),
    #[error(transparent)]
    AbiFailed(get_prop::Error),
    #[error("{0:?} isn't a valid target ABI.")]
    AbiInvalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        let msg = "Failed to detect connected Android devices";
        match self {
            Self::DevicesFailed(err) => err.report("Failed to run `adb devices`"),
            Self::NameFailed(err) => err.report(),
            Self::ModelFailed(err) | Self::AbiFailed(err) => err.report(),
            Self::AbiInvalid(_) => Report::error(msg, self),
            Self::Io(err) => Report::error(msg, err),
        }
    }
}

const ADB_DEVICE_REGEX: &str = r"^([\S]{6,100})	device\b";

pub fn device_list(env: &Env) -> Result<BTreeSet<Device<'static>>, Error> {
    let mut cmd = Command::new(env.platform_tools_path().join("adb"));
    cmd.arg("devices").envs(env.explicit_env());

    super::check_authorized(&cmd.output()?)
        .map(|raw_list| {
            regex_multi_line!(ADB_DEVICE_REGEX)
                .captures_iter(&raw_list)
                .map(|caps| {
                    assert_eq!(caps.len(), 2);
                    let serial_no = caps.get(1).unwrap().as_str().to_owned();
                    let model = get_prop(env, &serial_no, "ro.product.model")
                        .map_err(Error::ModelFailed)?;
                    let name = device_name(env, &serial_no).unwrap_or_else(|_| model.clone());
                    let abi = get_prop(env, &serial_no, "ro.product.cpu.abi")
                        .map_err(Error::AbiFailed)?;
                    let target =
                        Target::for_abi(&abi).ok_or_else(|| Error::AbiInvalid(abi.clone()))?;
                    Ok(Device::new(serial_no, name, model, target))
                })
                .collect()
        })
        .map_err(Error::DevicesFailed)?
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest(input, devices,
        case("* daemon not running; starting now at tcp:5020\n\
            * daemon started successfully\n\
            List of devices attached\n\
            AB1234DEFG\tdevice\n\
            192.168.100.103:55555\tdevice\n\
            ", vec!["AB1234DEFG", "192.168.100.103:55555"]
        ),
        case("List of devices attached \n", vec![]),
        case("** daemon not running; starting now at tcp:5037\n\
            * daemon started successfully\n\
            List of devices attached\n\
            emulator-5556	device product:sdk_google_phone_x86_64 model:Android_SDK_built_for_x86_64 device:generic_x86_64\n\
            emulator-5554	device product:sdk_google_phone_x86 model:Android_SDK_built_for_x86 device:generic_x86\n\
            0a388e93	device usb:1-1 product:razor model:Nexus_7 device:flo\n\
            ", vec!["emulator-5556", "emulator-5554", "0a388e93"]
        ),

    )]
    fn test_adb_output_regex(input: &str, devices: Vec<&'static str>) {
        let regex = regex_multi_line!(ADB_DEVICE_REGEX);
        println!("{}", input);
        let captures = regex
            .captures_iter(input)
            .map(|x| x.get(1).unwrap().as_str())
            .collect::<Vec<_>>();
        assert_eq!(captures, devices);
    }
}
