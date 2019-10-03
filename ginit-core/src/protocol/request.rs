use super::{ConfigError, PluginType, ResponseError, ResponseMsg, VERSION};
use crate::{
    cli::CliInput,
    config::{Config, ConfigTrait},
    opts, PluginTrait,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Request<'a> {
    pub plugin_name: &'a str,
    pub msg: RequestMsg,
}

impl<'a> Request<'a> {
    pub fn new(plugin_name: &'a str, msg: RequestMsg) -> Self {
        Self { plugin_name, msg }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum RequestMsg {
    Hello,
    Goodbye,
    Config {
        shared_config: Config,
        plugin_config: Option<Vec<u8>>,
    },
    Cli,
    Init,
    Exec {
        input: CliInput,
        noise_level: opts::NoiseLevel,
    },
}

impl RequestMsg {
    pub fn ty(&self) -> &'static str {
        match self {
            Self::Hello => "Hello",
            Self::Goodbye => "Goodbye",
            Self::Config { .. } => "Config",
            Self::Cli => "Cli",
            Self::Init => "Init",
            Self::Exec { .. } => "Exec",
        }
    }

    pub fn respond<P: PluginTrait>(self, plugin: &mut P) -> Result<ResponseMsg, ResponseError<P>> {
        match self {
            Self::Hello => Ok(ResponseMsg::Hello {
                protocol_version: VERSION,
                plugin_type: PluginType::Basic,
                description: P::DESCRIPTION.to_owned(),
            }),
            Self::Goodbye => Ok(ResponseMsg::Goodbye),
            Self::Config {
                shared_config,
                plugin_config,
            } => {
                let raw = plugin_config
                    .map(|plugin_config| {
                        toml::from_slice::<<P::Config as ConfigTrait>::Raw>(&plugin_config).map_err(
                            |err| ResponseError::ConfigFailed(ConfigError::ParseFailed(err)),
                        )
                    })
                    .transpose()?;
                let config =
                    <P::Config as ConfigTrait>::from_raw(shared_config, raw).map_err(|err| {
                        ResponseError::ConfigFailed(ConfigError::ValidationFailed(err))
                    })?;
                plugin.update_config(config);
                Ok(ResponseMsg::Config)
            }
            Self::Cli => Ok(ResponseMsg::Cli { cli: plugin.cli() }),
            Self::Init => {
                plugin.init().map_err(ResponseError::InitFailed)?;
                Ok(ResponseMsg::Init)
            }
            Self::Exec { input, noise_level } => {
                plugin
                    .exec(input, noise_level)
                    .map_err(ResponseError::ExecFailed)?;
                Ok(ResponseMsg::Exec)
            }
        }
    }
}
