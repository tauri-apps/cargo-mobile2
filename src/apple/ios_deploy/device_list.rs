use super::{DeviceInfo, Event};
use crate::{
    apple::{device::Device, target::Target},
    env::{Env, ExplicitEnv as _},
    util::cli::{Report, Reportable},
};
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum DeviceListError {
    DetectionFailed(bossy::Error),
    InvalidUtf8(std::str::Utf8Error),
    ArchInvalid(String),
}

impl Reportable for DeviceListError {
    fn report(&self) -> Report {
        let msg = "Failed to detect connected iOS devices";
        match self {
            Self::DetectionFailed(err) => Report::error(
                msg,
                format!("Failed to request device list from `ios-deploy`: {}", err),
            ),
            Self::InvalidUtf8(err) => {
                Report::error(msg, format!("Device info contained invalid UTF-8: {}", err))
            }
            Self::ArchInvalid(arch) => {
                Report::error(msg, format!("{:?} isn't a valid target arch.", arch))
            }
        }
    }
}

fn parse_device_list<'a>(output: &bossy::Output) -> Result<BTreeSet<Device<'a>>, DeviceListError> {
    Event::parse_list(output.stdout_str().map_err(DeviceListError::InvalidUtf8)?)
        .into_iter()
        .flat_map(|event| event.device_info().cloned())
        .map(
            |DeviceInfo {
                 device_identifier,
                 device_name,
                 model_arch,
                 model_name,
             }| {
                Target::for_arch(&model_arch)
                    .map(|target| Device::new(device_identifier, device_name, model_name, target))
                    .ok_or_else(|| DeviceListError::ArchInvalid(model_arch))
            },
        )
        .collect::<Result<_, _>>()
}

pub fn device_list<'a>(env: &Env) -> Result<BTreeSet<Device<'a>>, DeviceListError> {
    parse_device_list(
        &bossy::Command::pure_parse("ios-deploy --detect --timeout 1 --json --no-wifi")
            .unwrap()
            .with_env_vars(env.explicit_env())
            .with_stdout_piped()
            .with_stderr_piped()
            .run_and_wait_for_output()
            .map_err(DeviceListError::DetectionFailed)?,
    )
}
