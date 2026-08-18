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
    pairing_state: Option<String>,
    tunnel_state: Option<String>,
}

// Xcode 27+ consolidated `properties` dictionary, replacing the deprecated
// `hardwareProperties`, `deviceProperties` and `connectionProperties` fields
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Properties {
    connection: Option<PropertiesConnection>,
    hardware: Option<PropertiesHardware>,
    state: Option<PropertiesState>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertiesConnection {
    pairing_state: Option<String>,
    state: Option<String>,
}

// unlike the deprecated `hardwareProperties.cpuType`, this one has no `name`
#[derive(Debug, Deserialize)]
pub struct PropertiesCpuType {
    #[serde(rename = "type")]
    ty: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertiesHardware {
    udid: Option<String>,
    platform: Option<String>,
    product_type: Option<String>,
    cpu_type: Option<PropertiesCpuType>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertiesState {
    name: Option<String>,
}

/// mach-o CPU_TYPE_ARM64 (CPU_TYPE_ARM | CPU_ARCH_ABI64)
const CPU_TYPE_ARM64: i64 = 0x0100_000C;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaybeDeviceListDevice {
    connection_properties: Option<ConnectionProperties>,
    device_properties: Option<DeviceProperties>,
    hardware_properties: Option<MaybeHardwareProperties>,
    properties: Option<Properties>,
    // Xcode 27+ also lists simulators ("simulators"); physical devices are "default"
    visibility_class: Option<String>,
}

struct ResolvedDevice {
    name: String,
    udid: String,
    platform: String,
    product_type: String,
    paired: bool,
    arm64: bool,
}

impl MaybeDeviceListDevice {
    fn connection(&self) -> Option<&PropertiesConnection> {
        self.properties
            .as_ref()
            .and_then(|properties| properties.connection.as_ref())
    }

    fn legacy_hardware(&self) -> Option<&HardwareProperties> {
        match self.hardware_properties.as_ref() {
            Some(MaybeHardwareProperties::KnownProperties(hardware)) => Some(hardware),
            _ => None,
        }
    }

    fn hardware(&self) -> Option<&PropertiesHardware> {
        self.properties
            .as_ref()
            .and_then(|properties| properties.hardware.as_ref())
    }

    fn tunnel_state(&self) -> Option<&str> {
        self.connection_properties
            .as_ref()
            .and_then(|connection| connection.tunnel_state.as_deref())
            .or_else(|| {
                self.connection()
                    .and_then(|connection| connection.state.as_deref())
            })
    }

    fn resolve(&self) -> Option<ResolvedDevice> {
        let name = self
            .device_properties
            .as_ref()
            .and_then(|device| device.name.clone())
            .or_else(|| {
                self.properties
                    .as_ref()
                    .and_then(|properties| properties.state.as_ref())
                    .and_then(|state| state.name.clone())
            })?;
        let udid = self
            .legacy_hardware()
            .map(|hardware| hardware.udid.clone())
            .or_else(|| self.hardware().and_then(|hardware| hardware.udid.clone()))?;
        let platform = self
            .legacy_hardware()
            .map(|hardware| hardware.platform.clone())
            .or_else(|| {
                self.hardware()
                    .and_then(|hardware| hardware.platform.clone())
            })?;
        let product_type = self
            .legacy_hardware()
            .map(|hardware| hardware.product_type.clone())
            .or_else(|| {
                self.hardware()
                    .and_then(|hardware| hardware.product_type.clone())
            })?;
        let arm64 = self
            .legacy_hardware()
            .map(|hardware| hardware.cpu_type.name.starts_with("arm64"))
            .or_else(|| {
                self.hardware()
                    .and_then(|hardware| hardware.cpu_type.as_ref())
                    .and_then(|cpu_type| cpu_type.ty)
                    .map(|ty| ty == CPU_TYPE_ARM64)
            })
            .unwrap_or(false);
        let paired = self
            .connection_properties
            .as_ref()
            .and_then(|connection| connection.pairing_state.as_deref())
            .or_else(|| {
                self.connection()
                    .and_then(|connection| connection.pairing_state.as_deref())
            })
            == Some("paired");

        Some(ResolvedDevice {
            name,
            udid,
            platform,
            product_type,
            paired,
            arm64,
        })
    }
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
        .filter(|device| device.tunnel_state() != Some("unavailable"))
        // Xcode 27+ includes simulators in `devicectl list devices`; those are listed via simctl instead
        .filter(|device| device.visibility_class.as_deref() != Some("simulators"))
        .filter_map(|device| {
            let resolved = device.resolve();
            if resolved.is_none() {
                log::warn!("skipping device {device:?}, missing device or hardware properties");
            }
            resolved
        })
        .filter(|device| device.platform.contains("iOS") || device.platform.contains("xrOS"))
        .map(|device| {
            Device::new(
                device.udid,
                device.name,
                device.product_type,
                if device.arm64 {
                    Target::for_arch("arm64")
                } else {
                    Target::for_arch("x86_64")
                }
                .expect("invalid target arch"),
                DeviceKind::DeviceCtlDevice,
            )
            .paired(device.paired)
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
    fn can_deserialize_properties_only() {
        // Xcode 27+ deprecates hardwareProperties/deviceProperties/connectionProperties
        // in favor of a consolidated `properties` dictionary; this is the output shape
        // with `--omit-deprecated-fields-in-json` (and of future devicectl releases)
        let json = serde_json::json!({
            "result" : {
                "devices" : [
                    {
                        "identifier" : "00008101-000A0C123456001E",
                        "properties" : {
                            "connection" : {
                                "authenticationType" : "manualPairing",
                                "pairingState" : "paired",
                                "state" : "connected",
                                "transportType" : "localNetwork"
                            },
                            "hardware" : {
                                "cpuType" : {
                                    "subtype" : 2,
                                    "type" : 16777228
                                },
                                "deviceType" : "iPhone",
                                "marketingName" : "iPhone 15 Pro",
                                "platform" : "iOS",
                                "productType" : "iPhone16,1",
                                "reality" : "physical",
                                "udid" : "00008101-000A0C123456001E"
                            },
                            "state" : {
                                "bootState" : "booted",
                                "name" : "Tauri iPhone",
                                "visibilityClass" : "default"
                            }
                        },
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
    fn filters_simulators() {
        // Xcode 27+ devicectl (jsonVersion 5) lists simulators alongside physical devices
        let json = serde_json::json!({
            "result" : {
                "devices" : [
                    {
                        "connectionProperties" : {
                            "authenticationType" : "implicit",
                            "isMobileDeviceOnly" : false,
                            "lastConnectionDate" : "2026-08-17T18:19:48.000Z",
                            "pairingState" : "paired",
                            "potentialHostnames" : [],
                            "transportType" : "sameMachine",
                            "tunnelState" : "disconnected"
                        },
                        "deviceProperties" : {
                            "bootState" : "shutdown",
                            "ddiServicesAvailable" : false,
                            "hasInternalOSBuild" : false,
                            "name" : "iPhone 17 Pro",
                            "osBuildUpdate" : "24A5408d",
                            "osVersionNumber" : "27.0",
                            "provider" : "com.apple.CoreSimulator.SimulatorCoreDevicePlugin"
                        },
                        "hardwareProperties" : {
                            "cpuType" : {
                                "name" : "arm64",
                                "subType" : 0,
                                "type" : 16777228
                            },
                            "deviceType" : "iPhone",
                            "hardwareModel" : "V53",
                            "marketingName" : "iPhone 17 Pro",
                            "platform" : "iOS",
                            "productType" : "iPhone18,1",
                            "reality" : "simulated",
                            "udid" : "E60E9C71-FCE3-4257-8E2C-9413E8D82B5C"
                        },
                        "identifier" : "E60E9C71-FCE3-4257-8E2C-9413E8D82B5C",
                        "visibilityClass" : "simulators"
                    }
                ]
            },
        });

        let list = super::parse_device_list(serde_json::to_string(&json).unwrap())
            .expect("can deserialize");
        assert!(list.is_empty());
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
