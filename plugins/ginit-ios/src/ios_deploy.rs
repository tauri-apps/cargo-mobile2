use crate::{device::Device, target::Target};
use ginit_core::{
    env::Env,
    exports::into_result::{command::CommandError, IntoResult as _},
    util::pure_command::PureCommand,
};
use serde::Deserialize;
use std::{collections::BTreeSet, fmt, process::Stdio};

#[derive(Debug)]
pub enum DeviceListError {
    DetectionFailed(CommandError),
    KillFailed(std::io::Error),
    OutputFailed(std::io::Error),
    InvalidUtf8(std::str::Utf8Error),
    ParseFailed(serde_json::error::Error),
    ArchInvalid(String),
}

impl fmt::Display for DeviceListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceListError::DetectionFailed(err) => write!(
                f,
                "Failed to request device list from `ios-deploy`: {}",
                err
            ),
            DeviceListError::KillFailed(err) => write!(f, "Failed to kill `ios-deploy`: {}", err),
            DeviceListError::OutputFailed(err) => write!(
                f,
                "Failed to get device list output from `ios-deploy`: {}",
                err
            ),
            DeviceListError::InvalidUtf8(err) => {
                write!(f, "Device info contained invalid UTF-8: {}", err)
            }
            DeviceListError::ParseFailed(err) => {
                write!(f, "Device info couldn't be parsed: {}", err)
            }
            DeviceListError::ArchInvalid(arch) => {
                write!(f, "{:?} isn't a valid target arch.", arch)
            }
        }
    }
}

pub fn device_list<'a>(env: &Env) -> Result<BTreeSet<Device<'a>>, DeviceListError> {
    #[derive(Debug, Deserialize)]
    struct DeviceInfo {
        #[serde(rename = "DeviceIdentifier")]
        device_identifier: String,
        #[serde(rename = "DeviceName")]
        device_name: String,
        #[serde(rename = "modelArch")]
        model_arch: String,
        #[serde(rename = "modelName")]
        model_name: String,
    }
    #[derive(Debug, Deserialize)]
    struct DeviceDetected {
        #[serde(rename = "Device")]
        device: DeviceInfo,
    }
    let mut handle = PureCommand::new("ios-deploy", env)
        .args(&["--detect", "--json", "--no-wifi", "--unbuffered"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .into_result()
        .map_err(DeviceListError::DetectionFailed)?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    handle.kill().map_err(DeviceListError::KillFailed)?;
    let output = handle
        .wait_with_output()
        .map_err(DeviceListError::OutputFailed)?;
    let raw_list = std::str::from_utf8(&output.stdout).map_err(DeviceListError::InvalidUtf8)?;
    let raw_docs = {
        let mut raw_docs = Vec::new();
        let mut prev_index = 0;
        for (index, _) in raw_list.match_indices("}{") {
            let end = index + 1;
            raw_docs.push(&raw_list[prev_index..end]);
            prev_index = end;
        }
        raw_docs.push(&raw_list[prev_index..]);
        raw_docs
    };
    raw_docs
        .into_iter()
        .filter(|raw_doc| !raw_doc.is_empty())
        .map(|raw_doc| {
            serde_json::from_str::<DeviceDetected>(raw_doc)
                .map_err(DeviceListError::ParseFailed)
                .map(|device_detected| device_detected.device)
                .and_then(
                    |DeviceInfo {
                         device_identifier,
                         device_name,
                         model_arch,
                         model_name,
                     }| {
                        Target::for_arch(&model_arch)
                            .map(|target| {
                                Device::new(device_identifier, device_name, model_name, target)
                            })
                            .ok_or_else(|| DeviceListError::ArchInvalid(model_arch))
                    },
                )
        })
        .collect::<Result<_, _>>()
}
