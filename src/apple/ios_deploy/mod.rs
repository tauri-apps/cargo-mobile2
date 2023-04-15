mod device_list;
mod run;

pub use self::{device_list::*, run::*};

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
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
#[serde(tag = "Event")]
enum Event {
    #[serde(rename_all = "PascalCase")]
    #[allow(dead_code)]
    BundleCopy {
        percent: u32,
        overall_percent: u32,
        path: PathBuf,
    },
    #[serde(rename_all = "PascalCase")]
    #[allow(dead_code)]
    BundleInstall {
        percent: u32,
        overall_percent: u32,
        status: String,
    },
    #[serde(rename_all = "PascalCase")]
    DeviceDetected { device: DeviceInfo },
    #[serde(rename_all = "PascalCase")]
    #[allow(dead_code)]
    Error { code: u32, status: String },
    #[serde(other)]
    Unknown,
}

impl Event {
    fn parse_list(s: &str) -> Vec<Self> {
        fn parse_and_push(s: &str, docs: &mut Vec<Event>) {
            if !s.is_empty() {
                match serde_json::from_str(s) {
                    Ok(event) => {
                        log::debug!("parsed `ios-deploy` event: {:#?}", event);
                        docs.push(event)
                    }
                    Err(err) => {
                        log::error!(
                            "failed to parse `ios-deploy` event: {}\nraw event text:\n{}",
                            err,
                            s
                        );
                    }
                }
            }
        }

        let (mut docs, prev_index) =
            s.match_indices("}{")
                .fold((Vec::new(), 0), |(mut docs, prev_index), (index, _)| {
                    let end = index + 1;
                    parse_and_push(&s[prev_index..end], &mut docs);
                    (docs, end)
                });
        parse_and_push(&s[prev_index..], &mut docs);
        docs
    }

    fn device_info(&self) -> Option<&DeviceInfo> {
        if let Self::DeviceDetected { device } = self {
            Some(device)
        } else {
            None
        }
    }
}
