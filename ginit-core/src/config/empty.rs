use super::{shared::Shared, ConfigTrait, DefaultConfigTrait, RequiredConfigTrait};
use crate::util;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Raw {}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(skip_serializing)]
    shared: Shared,
}

impl ConfigTrait for Config {
    type DefaultConfig = DefaultConfig;

    type Raw = Raw;
    type Error = util::Never;
    fn from_raw(shared: Shared, _raw: Option<Self::Raw>) -> Result<Self, Self::Error> {
        Ok(Self { shared })
    }

    fn shared(&self) -> &Shared {
        &self.shared
    }
}

#[derive(Debug)]
pub struct DefaultConfig;

impl DefaultConfigTrait for DefaultConfig {
    type DetectError = util::Never;
    fn detect() -> Result<Self, Self::DetectError> {
        Ok(Self)
    }

    type RequiredConfig = RequiredConfig;
    type UpgradeError = util::Never;
    fn upgrade(self) -> Result<Self::RequiredConfig, Self::UpgradeError> {
        Ok(RequiredConfig)
    }
}

#[derive(Debug, Serialize)]
pub struct RequiredConfig;

impl RequiredConfigTrait for RequiredConfig {
    type PromptError = util::Never;
    fn prompt(_wrapper: &util::TextWrapper) -> Result<Self, Self::PromptError> {
        Ok(Self)
    }
}
