use super::{device::Device, target::Target};
use crate::env::{Env, ExplicitEnv as _};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fmt::{self, Display},
};

#[derive(Debug)]
pub enum DeviceListError {
    DetectionFailed(bossy::Error),
    KillFailed(std::io::Error),
    OutputFailed(bossy::Error),
    InvalidUtf8(std::str::Utf8Error),
    ParseFailed(serde_json::error::Error),
    ArchInvalid(String),
}

impl Display for DeviceListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DetectionFailed(err) => write!(
                f,
                "Failed to request device list from `ios-deploy`: {}",
                err
            ),
            Self::KillFailed(err) => write!(f, "Failed to kill `ios-deploy`: {}", err),
            Self::OutputFailed(err) => write!(
                f,
                "Failed to get device list output from `ios-deploy`: {}",
                err
            ),
            Self::InvalidUtf8(err) => write!(f, "Device info contained invalid UTF-8: {}", err),
            Self::ParseFailed(err) => write!(f, "Device info couldn't be parsed: {}", err),
            Self::ArchInvalid(arch) => write!(f, "{:?} isn't a valid target arch.", arch),
        }
    }
}

fn parse_device_list<'a>(output: &bossy::Output) -> Result<BTreeSet<Device<'a>>, DeviceListError> {
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

    let raw_list = output.stdout_str().map_err(DeviceListError::InvalidUtf8)?;
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

pub fn device_list<'a>(env: &Env) -> Result<BTreeSet<Device<'a>>, DeviceListError> {
    let mut handle = bossy::Command::pure("ios-deploy")
        .with_env_vars(env.explicit_env())
        .with_args(&["--detect", "--json", "--no-wifi", "--unbuffered"])
        .with_stdout_piped()
        .with_stderr_piped()
        .run()
        .map_err(DeviceListError::DetectionFailed)?;
    // TODO: this feels so gross
    std::thread::sleep(std::time::Duration::from_millis(500));
    handle.kill().map_err(DeviceListError::KillFailed)?;
    let result = handle.wait_for_output();
    log::debug!("`ios-deploy` device list result: {:#?}", result);
    match result {
        // This should actually never be `Ok`, since we killed it...
        Ok(output) => parse_device_list(&output),
        Err(err) => {
            let output = err.output().expect("`bossy::Handle::wait_for_output` failed with a `bossy::Cause` variant other than `CommandFailedWithOutput`");
            if output.stderr().is_empty() {
                parse_device_list(output)
            } else {
                Err(err).map_err(DeviceListError::OutputFailed)
            }
        }
    }
}
