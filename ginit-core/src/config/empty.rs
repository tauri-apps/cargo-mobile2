use super::{ConfigTrait, DefaultConfigTrait, RequiredConfigTrait, Shared};
use crate::util;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EmptyRaw {}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EmptyConfig {
    #[serde(skip_serializing)]
    shared: Shared,
}

impl ConfigTrait for EmptyConfig {
    type DefaultConfig = EmptyDefaultConfig;

    type Raw = EmptyRaw;
    type Error = util::Never;
    fn from_raw(shared: Shared, _raw: Option<Self::Raw>) -> Result<Self, Self::Error> {
        Ok(Self { shared })
    }

    fn shared(&self) -> &Shared {
        &self.shared
    }
}

#[derive(Debug)]
pub struct EmptyDefaultConfig;

impl DefaultConfigTrait for EmptyDefaultConfig {
    type DetectError = util::Never;
    fn detect() -> Result<Self, Self::DetectError> {
        Ok(Self)
    }

    type RequiredConfig = EmptyRequiredConfig;
    type UpgradeError = util::Never;
    fn upgrade(self) -> Result<Self::RequiredConfig, Self::UpgradeError> {
        Ok(EmptyRequiredConfig)
    }
}

#[derive(Debug, Serialize)]
pub struct EmptyRequiredConfig;

impl RequiredConfigTrait for EmptyRequiredConfig {
    type PromptError = util::Never;
    fn prompt(_wrapper: &util::TextWrapper) -> Result<Self, Self::PromptError> {
        Ok(Self)
    }
}
