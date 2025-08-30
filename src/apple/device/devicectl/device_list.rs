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
use std::{collections::BTreeSet, env::temp_dir, fs::read_to_string, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceListError {
    #[error("Failed to request device list from `{command}`: {error}")]
    DetectionFailed {
        command: String,
        error: std::io::Error,
    },
    #[error("`simctl list` returned an invalid JSON: {0}")]
    InvalidDeviceList(#[from] serde_json::Error),
    #[error("Failed to read file {path:?}: {error}")]
    ReadFile {
        path: PathBuf,
        error: std::io::Error,
    },
    #[error("Failed to write file {path:?}: {error}")]
    WriteFile {
        path: PathBuf,
        error: std::io::Error,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProperties {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuType {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareProperties {
    udid: String,
    platform: String,
    product_type: String,
    cpu_type: CpuType,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MaybeHardwareProperties {
    // order matters, if we can deserialize to HardwareProperties we get Self::KnownProperties, fallback to Self::Invalid
    KnownProperties(HardwareProperties),
    #[allow(dead_code)]
    Invalid(serde_json::Value),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProperties {
    pairing_state: String,
    tunnel_state: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaybeDeviceListDevice {
    connection_properties: ConnectionProperties,
    device_properties: DeviceProperties,
    hardware_properties: MaybeHardwareProperties,
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
    devices: Vec<MaybeDeviceListDevice>,
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

fn parse_device_list<'a>(json: String) -> Result<BTreeSet<Device<'a>>, DeviceListError> {
    let devices = serde_json::from_str::<DeviceListOutput>(&json)?
        .result
        .devices
        .into_iter()
        // early filter to not log devices with missing hardwareProperties unless they are actually in a weird/unknown state
        .filter(|device| device.connection_properties.tunnel_state != "unavailable")
        .filter_map(|device| {
            if let MaybeHardwareProperties::KnownProperties(hardware_properties) =
                device.hardware_properties
            {
                Some(DeviceListDevice {
                    connection_properties: device.connection_properties,
                    device_properties: device.device_properties,
                    hardware_properties,
                })
            } else {
                log::warn!("skipping device {:?}, unknown hardwareProperties", device);
                None
            }
        })
        .filter(|device| {
            device.hardware_properties.platform.contains("iOS")
                || device.hardware_properties.platform.contains("xrOS")
        })
        .filter(|device| device.device_properties.name.is_some())
        .map(|device| {
            Device::new(
                device.hardware_properties.udid,
                device
                    .device_properties
                    .name
                    .expect("empty device name was filtered"),
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
    std::fs::write(&json_output_path, "").map_err(|err| DeviceListError::WriteFile {
        path: json_output_path.clone(),
        error: err,
    })?;

    let cmd = duct::cmd("xcrun", ["devicectl", "list", "devices", "--json-output"])
        .before_spawn(move |cmd| {
            cmd.arg(&json_output_path);
            Ok(())
        })
        .stderr_capture()
        .stdout_capture()
        .vars(env.explicit_env());

    cmd.run().map_err(|err| DeviceListError::DetectionFailed {
        command: format!("{cmd:?}"),
        error: err,
    })?;

    let contents = read_to_string(&json_output_path_).map_err(|err| DeviceListError::ReadFile {
        path: json_output_path_,
        error: err,
    })?;
    parse_device_list(contents)
}

#[cfg(test)]
mod tests {
    #[test]
    fn can_deserialize_device_list() {
        let json = serde_json::json!({
            "result" : {
                "devices" : [
                    {
                        "capabilities" : [
                        {
                            "featureIdentifier" : "com.apple.coredevice.feature.tags",
                            "name" : "Modify Tags"
                        }
                        ],
                        "connectionProperties" : {
                        "isMobileDeviceOnly" : false,
                        "pairingState" : "unpaired",
                        "potentialHostnames" : [

                        ],
                        "tunnelState" : "connected"
                        },
                        "deviceProperties" : {
                            "name": "Tauri iPhone",
                            "bootState" : "booted",
                            "ddiServicesAvailable" : false
                        },
                        "hardwareProperties" : {
                            "udid": "781BF0DD-XXXXXXXXXXXXXXX",
                            "platform": "iOS",
                            "productType": "iOS",
                            "cpuType": {
                                "name": "arm64"
                            },
                        },
                        "identifier" : "781BF0DD-XXXXXXXXXXXXXXX",
                        "tags" : [],
                    "visibilityClass" : "default"
                    }
                ]
            },
        });

        let list = super::parse_device_list(serde_json::to_string(&json).unwrap())
            .expect("can deserialize");
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn can_deserialize_empty_hardware_properties() {
        let json = serde_json::json!({
            "result" : {
                "devices" : [
                    {
                        "capabilities" : [
                        {
                            "featureIdentifier" : "com.apple.coredevice.feature.tags",
                            "name" : "Modify Tags"
                        }
                        ],
                        "connectionProperties" : {
                        "isMobileDeviceOnly" : false,
                        "pairingState" : "unpaired",
                        "potentialHostnames" : [

                        ],
                        "tunnelState" : "connected"
                        },
                        "deviceProperties" : {
                            "name": "Tauri iPhone",
                            "bootState" : "booted",
                            "ddiServicesAvailable" : false
                        },
                        "hardwareProperties" : {

                        },
                        "identifier" : "781BF0DD-XXXXXXXXXXXXXXX",
                        "tags" : [],
                    "visibilityClass" : "default"
                    }
                ]
            },
        });

        let list = super::parse_device_list(serde_json::to_string(&json).unwrap())
            .expect("can deserialize");
        assert!(list.is_empty());
    }

    #[test]
    fn filters_empty_device_name() {
        let json = serde_json::json!({
            "result" : {
                "devices" : [
                    {
                        "capabilities" : [
                        {
                            "featureIdentifier" : "com.apple.coredevice.feature.tags",
                            "name" : "Modify Tags"
                        }
                        ],
                        "connectionProperties" : {
                        "isMobileDeviceOnly" : false,
                        "pairingState" : "unpaired",
                        "potentialHostnames" : [

                        ],
                        "tunnelState" : "connected"
                        },
                        "deviceProperties" : {
                            "bootState" : "booted",
                            "ddiServicesAvailable" : false
                        },
                        "hardwareProperties" : {
                            "udid": "781BF0DD-XXXXXXXXXXXXXXX",
                            "platform": "iOS",
                            "productType": "iOS",
                            "cpuType": {
                                "name": "arm64"
                            },
                        },
                        "identifier" : "781BF0DD-XXXXXXXXXXXXXXX",
                        "tags" : [],
                    "visibilityClass" : "default"
                    }
                ]
            },
        });

        let list = super::parse_device_list(serde_json::to_string(&json).unwrap())
            .expect("can deserialize");
        assert!(list.is_empty());
    }
}
