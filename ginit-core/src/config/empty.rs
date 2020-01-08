use super::{shared::Shared, ConfigTrait, DetectedConfigTrait, RawConfigTrait};
use crate::util;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(skip_serializing)]
    shared: Shared,
}

impl ConfigTrait for Config {
    type Raw = Raw;
    type Error = util::Never;
    fn from_raw(shared: Shared, _raw: Option<Self::Raw>) -> Result<Self, Self::Error> {
        Ok(Self { shared })
    }

    fn shared(&self) -> &Shared {
        &self.shared
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Raw;

impl RawConfigTrait for Raw {
    type Detected = Detected;

    type FromDetectedError = util::Never;
    fn from_detected(_detected: Self::Detected) -> Result<Self, Self::FromDetectedError> {
        Ok(Self)
    }

    type FromPromptError = util::Never;
    fn from_prompt(
        _detected: Self::Detected,
        _wrapper: &util::TextWrapper,
    ) -> Result<Self, Self::FromPromptError> {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct Detected;

impl DetectedConfigTrait for Detected {
    type Error = util::Never;
    fn new() -> Result<Self, Self::Error> {
        Ok(Self)
    }
}
