use crate::{
    apple::{
        device::{Device, DeviceKind},
        target::Target,
    },
    env::{Env, ExplicitEnv as _},
    util::cli::{Report, Reportable},
    DuctExpressionExt,
};
use serde::Deserialize;
use std::{collections::BTreeSet, env::temp_dir, fs::read_to_string};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceListError {
    #[error("Failed to request device list from `devicectl`: {0}")]
    DetectionFailed(#[from] std::io::Error),
    #[error("`simctl list` returned an invalid JSON: {0}")]
    InvalidDeviceList(#[from] serde_json::Error),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProperties {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuType {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareProperties {
    udid: String,
    platform: String,
    product_type: String,
    cpu_type: CpuType,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProperties {
    pairing_state: String,
    tunnel_state: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceListDevice {
    connection_properties: ConnectionProperties,
    device_properties: DeviceProperties,
    hardware_properties: HardwareProperties,
}

#[derive(Deserialize)]
struct DeviceListResult {
    devices: Vec<DeviceListDevice>,
}

#[derive(Deserialize)]
struct DeviceListOutput {
    result: DeviceListResult,
}

impl Reportable for DeviceListError {
    fn report(&self) -> Report {
        Report::error("Failed to detect connected iOS simulators", self)
    }
}

fn filter_invalid_devices(json_value: &mut serde_json::Value) {
    if let Some(result) = json_value.get_mut("result") {
        if let Some(devices) = result.get_mut("devices") {
            if let Some(devices_array) = devices.as_array_mut() {
                devices_array.retain(|device| {
                    if let Some(hardware_props) = device.get("hardwareProperties") {
                        !hardware_props.is_object() || !hardware_props.as_object().unwrap().is_empty()
                    } else {
                        false // Remove devices without hardwareProperties
                    }
                });
            }
        }
    }
}

fn parse_device_list<'a>(json: String) -> Result<BTreeSet<Device<'a>>, DeviceListError> {
    // First parse the JSON as a generic Value to filter out invalid devices
    let mut json_value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| {
            eprintln!("Error parsing initial JSON: {}", e);
            DeviceListError::InvalidDeviceList(e)
        })?;

    // Filter out devices with empty hardwareProperties
    filter_invalid_devices(&mut json_value);

    // Now parse the filtered JSON in a typed way
    let output = serde_json::from_value::<DeviceListOutput>(json_value)?;

    let devices = output
        .result
        .devices
        .into_iter()
        .filter(|device| {
            device.connection_properties.tunnel_state != "unavailable"
                && (device.hardware_properties.platform.contains("iOS")
                    || device.hardware_properties.platform.contains("xrOS"))
        })
        .map(|device| {
            Device::new(
                device.hardware_properties.udid,
                device.device_properties.name,
                device.hardware_properties.product_type,
                if device
                    .hardware_properties
                    .cpu_type
                    .name
                    .starts_with("arm64")
                {
                    Target::for_arch("arm64")
                } else {
                    Target::for_arch("x86_64")
                }
                .expect("invalid target arch"),
                DeviceKind::DeviceCtlDevice,
            )
            .paired(device.connection_properties.pairing_state == "paired")
        })
        .collect();

    Ok(devices)
}

pub fn device_list<'a>(env: &Env) -> Result<BTreeSet<Device<'a>>, DeviceListError> {
    let json_output_path = temp_dir().join("devicelist.json");
    let json_output_path_ = json_output_path.clone();
    std::fs::write(&json_output_path, "")?;

    duct::cmd("xcrun", ["devicectl", "list", "devices", "--json-output"])
        .before_spawn(move |cmd| {
            cmd.arg(&json_output_path);
            Ok(())
        })
        .stderr_capture()
        .stdout_capture()
        .vars(env.explicit_env())
        .run()
        .map_err(DeviceListError::DetectionFailed)?;

    let contents = read_to_string(json_output_path_)?;
    parse_device_list(contents)
}
