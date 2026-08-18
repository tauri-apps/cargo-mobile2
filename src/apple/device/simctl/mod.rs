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

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Platform {
    Ios,
    Xros,
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ios => write!(f, "iOS"),
            Self::Xros => write!(f, "xrOS"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Device {
    name: String,
    udid: String,
    platform: Platform,
    os_version: String,
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({} {})", self.name, self.platform, self.os_version)
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

    pub fn os_version(&self) -> &str {
        &self.os_version
    }

    // Xcode 27 removed Simulator.app in favor of Device Hub
    fn simulator_app_available(env: &Env) -> bool {
        duct::cmd("open", ["-Ra", "Simulator"])
            .vars(env.explicit_env())
            .stdout_null()
            .stderr_null()
            .unchecked()
            .run()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    // Device Hub does not boot a device on launch like Simulator.app does with
    // -CurrentDeviceUDID, so we must boot it via simctl and wait for it to finish
    fn boot(&self, env: &Env) -> std::io::Result<()> {
        duct::cmd("xcrun", ["simctl", "bootstatus", &self.udid, "-b"])
            .vars(env.explicit_env())
            .dup_stdio()
            .run()
            .map(|_| ())
    }

    fn open_simulator_command(&self, env: &Env) -> duct::Expression {
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

    fn open_device_hub_command(&self, env: &Env) -> duct::Expression {
        // deep link handled by DeviceKit's DeviceURLActionProvider, focusing this
        // device in Device Hub (launching the app first if needed)
        duct::cmd("open", [format!("devices://device/open?id={}", self.udid)])
            .vars(env.explicit_env())
            .dup_stdio()
    }

    pub fn start(&self, env: &Env) -> std::io::Result<duct::Handle> {
        if Self::simulator_app_available(env) {
            self.open_simulator_command(env).start()
        } else {
            self.boot(env)?;
            self.open_device_hub_command(env).start()
        }
    }
    pub fn start_detached(&self, env: &Env) -> std::io::Result<()> {
        if Self::simulator_app_available(env) {
            self.open_simulator_command(env).run_and_detach()
        } else {
            self.boot(env)?;
            self.open_device_hub_command(env).run_and_detach()
        }
    }
}
