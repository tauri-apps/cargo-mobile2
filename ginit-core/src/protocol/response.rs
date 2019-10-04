use super::Version;
use crate::{cli::Cli, config::ConfigTrait, PluginTrait};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Debug, Deserialize, Serialize)]
pub struct Response {
    pub status: Result<ResponseMsg, String>,
}

impl Response {
    pub fn new<P: PluginTrait>(status: Result<ResponseMsg, ResponseError<P>>) -> Self {
        Self {
            status: status.map_err(|err| format!("Plugin {:?} {}", P::NAME, err)),
        }
    }

    pub fn exit_requested(&self) -> bool {
        self.status
            .as_ref()
            .map(|msg| msg.exit_requested())
            .unwrap_or_default()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ResponseMsg {
    Hello {
        protocol_version: Version,
        features: Features,
        description: String,
    },
    Goodbye,
    Config,
    Cli {
        cli: Option<Cli>,
    },
    Exec,
    Init,
}

impl ResponseMsg {
    pub fn ty(&self) -> &'static str {
        match self {
            Self::Hello { .. } => "Hello",
            Self::Goodbye => "Goodbye",
            Self::Config => "Config",
            Self::Cli { .. } => "Cli",
            Self::Exec { .. } => "Exec",
            Self::Init => "Init",
        }
    }

    pub fn exit_requested(&self) -> bool {
        if let Self::Goodbye = self {
            true
        } else {
            false
        }
    }
}

bitflags::bitflags! {
    #[derive(Default, Deserialize, Serialize)]
    pub struct Features: u32 {
        const BASIC = 0b00000000;
        const TARGET = 0b00000001;
    }
}

#[derive(Debug)]
pub enum ConfigError<P: PluginTrait> {
    ParseFailed(toml::de::Error),
    ValidationFailed(<P::Config as ConfigTrait>::Error),
}

impl<P: PluginTrait> Display for ConfigError<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseFailed(err) => write!(f, "failed to parse config: {}", err),
            Self::ValidationFailed(err) => write!(f, "config invalid: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum ResponseError<P: PluginTrait> {
    ConfigFailed(ConfigError<P>),
    InitFailed(P::InitError),
    ExecFailed(P::ExecError),
}

impl<P: PluginTrait> Display for ResponseError<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigFailed(err) => write!(f, "{}", err),
            Self::InitFailed(err) => write!(f, "project init failed: {}", err),
            Self::ExecFailed(err) => write!(f, "command execution failed: {}", err),
        }
    }
}
