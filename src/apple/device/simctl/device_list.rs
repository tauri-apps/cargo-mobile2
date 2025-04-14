use super::{Device, Platform};
use crate::{
    env::{Env, ExplicitEnv as _},
    util::cli::{Report, Reportable},
    DuctExpressionExt,
};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceListError {
    #[error("Failed to request device list from `simctl`: {0}")]
    DetectionFailed(#[from] std::io::Error),
    #[error("`simctl list` returned an invalid JSON: {0}")]
    InvalidDeviceList(#[from] serde_json::Error),
}

#[derive(Deserialize)]
pub struct OutputDevice {
    name: String,
    udid: String,
}

#[derive(Deserialize)]
struct DeviceListOutput {
    devices: HashMap<String, Vec<OutputDevice>>,
}

impl Reportable for DeviceListError {
    fn report(&self) -> Report {
        Report::error("Failed to detect connected iOS simulators", self)
    }
}

fn parse_device_list(output: &std::process::Output) -> Result<BTreeSet<Device>, DeviceListError> {
    let stdout = String::from_utf8_lossy(&output.stdout);

    let devices = serde_json::from_str::<DeviceListOutput>(&stdout)?
        .devices
        .into_iter()
        .filter_map(|(k, devices)| {
            k.split_once("iOS-")
                .map(|(_, version)| (Platform::Ios, version.replace('-', ".")))
                .or_else(|| {
                    k.split_once("xrOS-")
                        .map(|(_, version)| (Platform::Xros, version.replace('-', ".")))
                })
                .map(|(platform, version)| {
                    devices
                        .into_iter()
                        .map(|device| Device {
                            name: device.name,
                            udid: device.udid,
                            platform,
                            os_version: version.clone(),
                        })
                        .collect::<Vec<_>>()
                })
        })
        .flatten()
        .collect();

    Ok(devices)
}

pub fn device_list(env: &Env) -> Result<BTreeSet<Device>, DeviceListError> {
    let result = duct::cmd(
        "xcrun",
        ["simctl", "list", "--json", "devices", "available"],
    )
    .vars(env.explicit_env())
    .stdout_capture()
    .stderr_capture()
    .run();
    match result {
        Ok(output) => {
            if output.stdout.is_empty() && output.stderr.is_empty() {
                log::info!("device detection returned a non-zero exit code, but stdout and stderr are both empty; interpreting as a successful run with no devices connected");
                Ok(Default::default())
            } else {
                parse_device_list(&output)
            }
        }
        Err(err) => Err(DeviceListError::DetectionFailed(err)),
    }
}
