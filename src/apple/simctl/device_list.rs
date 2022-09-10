use super::Device;
use crate::{
    bossy,
    env::{Env, ExplicitEnv as _},
    util::{Report, Reportable},
};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceListError {
    #[error("Failed to request device list from `simctl`: {0}")]
    DetectionFailed(#[from] bossy::Error),
    #[error("`simctl list` returned an invalid JSON: {0}")]
    InvalidDeviceList(#[from] serde_json::Error),
}

#[derive(Deserialize)]
struct DeviceListOutput {
    devices: HashMap<String, Vec<Device>>,
}

impl Reportable for DeviceListError {
    fn report(&self) -> Report {
        Report::error("Failed to detect connected iOS simulators", self)
    }
}

fn parse_device_list<'a>(output: &bossy::Output) -> Result<BTreeSet<Device>, DeviceListError> {
    let stdout = output.stdout_str()?;

    let devices = serde_json::from_str::<DeviceListOutput>(stdout)?
        .devices
        .into_iter()
        .filter(|(k, _)| k.contains("iOS"))
        .map(|(_, v)| v)
        .flatten()
        .collect();

    Ok(devices)
}

pub fn device_list<'a>(env: &Env) -> Result<BTreeSet<Device>, DeviceListError> {
    let result = bossy::Command::pure_parse("xcrun simctl list --json devices available")
        .with_env_vars(env.explicit_env())
        .run_and_wait_for_output();
    match result {
        Ok(output) => parse_device_list(&output),
        Err(err) => {
            let output = err
                .output()
                .expect("developer error: `simctl list` output wasn't collected");
            if output.stdout().is_empty() && output.stderr().is_empty() {
                log::info!("device detection returned a non-zero exit code, but stdout and stderr are both empty; interpreting as a successful run with no devices connected");
                Ok(Default::default())
            } else {
                Err(DeviceListError::DetectionFailed(err))
            }
        }
    }
}
