use crate::{opts, target::TargetTrait, TargetPluginTrait};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Cli {
    pub commands: Vec<Command>,
    pub target_info: Option<TargetInfo>,
    pub device_info: Option<DeviceInfo>,
}

impl Cli {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_command(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }

    pub fn with_target_info<'a, P: TargetPluginTrait<'a>>(mut self) -> Self {
        self.target_info = Some(TargetInfo::new::<P>());
        self
    }

    // pub fn with_device_info<'a, P: TargetPluginTrait<'a>>(mut self) -> Self {
    //     self.device_info = Some(DeviceInfo::new::<P>());
    //     self
    // }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Command {
    pub name: String,
    pub about: String,
    pub hidden: bool,
    pub args: Vec<Arg>,
}

impl Command {
    pub fn new(name: impl Into<String>, about: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            about: about.into(),
            hidden: false,
            args: vec![],
        }
    }

    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn with_arg(mut self, arg: Arg) -> Self {
        self.args.push(arg);
        self
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Arg {
    Custom {
        name: String,
        required: bool,
        index: Option<u64>,
    },
    FromUsage {
        usage: String,
    },
    TargetList,
    Device,
    Release,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TargetInfo {
    pub targets: Vec<String>,
    pub default: String,
}

impl TargetInfo {
    pub fn new<'a, P: TargetPluginTrait<'a>>() -> Self {
        Self {
            targets: <P::Target as TargetTrait>::all()
                .keys()
                .map(|key| key.to_string())
                .collect::<Vec<_>>(),
            default: <P::Target as TargetTrait>::DEFAULT_KEY.to_owned(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceInfo {
    pub devices: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CliInput {
    pub command: String,
    pub args: Vec<ArgInput>,
}

impl CliInput {
    pub fn targets(&self) -> Option<&[String]> {
        for arg in &self.args {
            if let ArgInput::TargetList { targets } = arg {
                return Some(targets);
            }
        }
        None
    }

    pub fn device(&self) -> Option<&str> {
        for arg in &self.args {
            if let ArgInput::Device { device } = arg {
                return Some(&device);
            }
        }
        None
    }

    pub fn profile(&self) -> Option<opts::Profile> {
        for arg in &self.args {
            if let ArgInput::Release { profile } = arg {
                return Some(*profile);
            }
        }
        None
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ArgInput {
    Custom {
        name: String,
        present: bool,
        value: Option<String>,
    },
    FromUsage {
        name: String,
        present: bool,
        value: Option<String>,
    },
    TargetList {
        targets: Vec<String>,
    },
    Device {
        device: String,
    },
    Release {
        profile: opts::Profile,
    },
}
