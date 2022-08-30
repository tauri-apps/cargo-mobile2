use super::target::Target;
use crate::apple::device::Device as AppleDevice;
use crate::{
    bossy,
    env::{Env, ExplicitEnv},
};
use serde::Deserialize;

use std::fmt::Display;

mod device_list;
mod run;

pub use device_list::{device_list, DeviceListError};
pub use run::{run, RunError};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Device {
    name: String,
    udid: String,
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<'a> From<Device> for AppleDevice<'a> {
    fn from(device: Device) -> AppleDevice<'a> {
        AppleDevice::new(
            device.udid,
            device.name,
            "".into(),
            Target::for_arch(if cfg!(target_arch = "aarch64") {
                "arm64-sim"
            } else {
                "x86_64"
            })
            .unwrap(),
        )
        .simulator()
    }
}

impl Device {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn start(&self, env: &Env) -> bossy::Result<bossy::Handle> {
        bossy::Command::impure("open")
            .with_args(&[
                "-a",
                "Simulator",
                "--args",
                "-CurrentDeviceUDID",
                &self.udid,
            ])
            .with_env_vars(env.explicit_env())
            .run()
    }
}
