mod common_email_providers;
pub mod name;
mod raw;

pub use self::raw::*;

use crate::util;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};

pub static KEY: &'static str = "app";

#[derive(Debug)]
pub enum Error {
    NameInvalid(name::Invalid),
    DomainInvalid { domain: String },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameInvalid(err) => write!(f, "`{}.name` invalid: {}", KEY, err),
            Self::DomainInvalid { domain } => write!(
                f,
                "`{}.domain` invalid: {:?} isn't valid domain syntax.",
                KEY, domain
            ),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct App {
    root_dir: PathBuf,
    name: String,
    stylized_name: String,
    domain: String,
}

impl App {
    pub fn from_raw(root_dir: PathBuf, raw: Raw) -> Result<Self, Error> {
        assert!(root_dir.is_absolute(), "root must be absolute");
        let name = name::validate(raw.name).map_err(Error::NameInvalid)?;
        let stylized_name = raw.stylized_name.unwrap_or_else(|| name.clone());
        let domain = {
            let domain = raw.domain;
            if publicsuffix::Domain::has_valid_syntax(&domain) {
                Ok(domain)
            } else {
                Err(Error::DomainInvalid { domain })
            }
        }?;
        Ok(Self {
            root_dir,
            name,
            stylized_name,
            domain,
        })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn prefix_path(&self, path: impl AsRef<Path>) -> PathBuf {
        util::prefix_path(self.root_dir(), path)
    }

    pub fn unprefix_path(&self, path: impl AsRef<Path>) -> Result<PathBuf, util::PathNotPrefixed> {
        util::unprefix_path(self.root_dir(), path)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn name_snake(&self) -> String {
        use heck::SnekCase as _;
        self.name().to_snek_case()
    }

    pub fn stylized_name(&self) -> &str {
        &self.stylized_name
    }

    pub fn reverse_domain(&self) -> String {
        self.domain
            .clone()
            .split('.')
            .rev()
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root_dir().join("Cargo.toml")
    }

    pub fn asset_dir(&self) -> PathBuf {
        self.root_dir().join("res")
    }
}
