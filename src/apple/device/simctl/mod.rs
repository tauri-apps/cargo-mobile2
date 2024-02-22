use super::super::target::Target;
use super::DeviceKind;
use crate::apple::device::Device as AppleDevice;
use crate::env::{Env, ExplicitEnv};
use crate::DuctExpressionExt;
use serde::Deserialize;

use std::fmt::Display;

mod device_list;
mod run;

pub use device_list::device_list;
pub use run::run;

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
            DeviceKind::Simulator,
        )
    }
}

impl Device {
    pub fn name(&self) -> &str {
        &self.name
    }

    fn command(&self, env: &Env) -> duct::Expression {
        duct::cmd(
            "open",
            [
                "-a",
                "Simulator",
                "--args",
                "-CurrentDeviceUDID",
                &self.udid,
            ],
        )
        .vars(env.explicit_env())
        .dup_stdio()
    }

    pub fn start(&self, env: &Env) -> std::io::Result<duct::Handle> {
        self.command(env).start()
    }
    pub fn start_detached(&self, env: &Env) -> std::io::Result<()> {
        self.command(env).run_and_detach()
    }
}
