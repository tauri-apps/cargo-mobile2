use super::{DeviceInfo, Event};
use crate::{
    apple::{device::Device, target::Target},
    env::{Env, ExplicitEnv as _},
    util::cli::{Report, Reportable},
};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceListError {
    #[error("Failed to request device list from `ios-deploy`: {0}")]
    DetectionFailed(#[from] bossy::Error),
    #[error("{0:?} isn't a valid target arch.")]
    ArchInvalid(String),
}

impl Reportable for DeviceListError {
    fn report(&self) -> Report {
        Report::error("Failed to detect connected iOS devices", self)
    }
}

fn parse_device_list<'a>(output: &bossy::Output) -> Result<BTreeSet<Device<'a>>, DeviceListError> {
    Event::parse_list(output.stdout_str()?)
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
    let result = bossy::Command::pure_parse("ios-deploy --detect --timeout 1 --json --no-wifi")
        .with_env_vars(env.explicit_env())
        .run_and_wait_for_output();
    match result {
        Ok(output) => parse_device_list(&output),
        Err(err) => {
            let output = err
                .output()
                .expect("developer error: `ios-deploy --detect` output wasn't collected");
            if output.stdout().is_empty() && output.stderr().is_empty() {
                log::info!("device detection returned a non-zero exit code, but stdout and stderr are both empty; interpreting as a successful run with no devices connected");
                Ok(Default::default())
            } else {
                Err(DeviceListError::DetectionFailed(err))
            }
        }
    }
}
