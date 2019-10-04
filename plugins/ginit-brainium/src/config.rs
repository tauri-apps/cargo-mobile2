use ginit_core::config::{Config as CoreConfig, ConfigTrait};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {}

impl Display for Error {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Raw {}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(skip_serializing)]
    shared: CoreConfig,
}

impl ConfigTrait for Config {
    type Raw = Raw;
    type Error = Error;

    fn from_raw(shared: CoreConfig, raw: Option<Self::Raw>) -> Result<Self, Self::Error> {
        let raw = raw.unwrap_or_default();
        Ok(Self { shared })
    }

    fn shared(&self) -> &CoreConfig {
        &self.shared
    }
}
