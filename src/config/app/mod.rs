mod common_email_providers;
pub mod name;
mod raw;

pub use self::raw::*;

use crate::{
    templating::{self, Pack},
    util::{self, cli::Report},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub static KEY: &'static str = "app";

pub static DEFAULT_ASSET_DIR: &'static str = "res";
#[cfg(feature = "brainium")]
pub static DEFAULT_TEMPLATE_PACK: &'static str = "rust-lib-app";
#[cfg(not(feature = "brainium"))]
pub static DEFAULT_TEMPLATE_PACK: &'static str = "wgpu-app";
#[cfg(feature = "brainium")]
pub static EXEMPT_TEMPLATE_PACKS: &'static [&'static str] = &["rust-lib-app", "rust-lib-minimal"];

#[derive(Debug)]
pub enum Error {
    NameInvalid(name::Invalid),
    DomainInvalid {
        domain: String,
    },
    AssetDirNormalizationFailed {
        asset_dir: PathBuf,
        cause: util::NormalizationError,
    },
    AssetDirOutsideOfAppRoot {
        asset_dir: PathBuf,
        root_dir: PathBuf,
    },
    TemplatePackNotFound(templating::LookupError),
}

impl Error {
    pub fn report(&self, msg: &str) -> Report {
        match self {
            Self::NameInvalid(err) => {
                Report::error(msg, format!("`{}.name` invalid: {}", KEY, err))
            }
            Self::DomainInvalid { domain } => Report::error(
                msg,
                format!(
                    "`{}.domain` invalid: {:?} isn't valid domain syntax",
                    KEY, domain
                ),
            ),
            Self::AssetDirNormalizationFailed { asset_dir, cause } => Report::error(
                msg,
                format!(
                    "`{}.asset-dir` {:?} couldn't be normalized: {}",
                    KEY, asset_dir, cause
                ),
            ),
            Self::AssetDirOutsideOfAppRoot {
                asset_dir,
                root_dir,
            } => Report::error(
                msg,
                format!(
                    "`{}.asset-dir` {:?} is outside of the app root {:?}",
                    KEY, asset_dir, root_dir,
                ),
            ),
            Self::TemplatePackNotFound(err) => Report::error(msg, err),
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
    asset_dir: PathBuf,
    template_pack: Pack,
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

        if raw.asset_dir.as_deref() == Some(DEFAULT_ASSET_DIR) {
            log::warn!(
                "`{}.asset-dir` is set to the default value; you can remove it from your config",
                KEY
            );
        }
        let asset_dir = raw.asset_dir.map(PathBuf::from).unwrap_or_else(|| {
            log::info!(
                "`{}.asset-dir` not set; defaulting to {}",
                KEY,
                DEFAULT_ASSET_DIR
            );
            DEFAULT_ASSET_DIR.into()
        });
        if !util::under_root(&asset_dir, &root_dir).map_err(|cause| {
            Error::AssetDirNormalizationFailed {
                asset_dir: asset_dir.clone(),
                cause,
            }
        })? {
            return Err(Error::AssetDirOutsideOfAppRoot {
                asset_dir,
                root_dir,
            });
        }

        #[cfg(feature = "brainium")]
        let template_pack = {
            if raw.template_pack.as_deref() == Some(DEFAULT_TEMPLATE_PACK) {
                log::warn!(
                    "`{}.template-pack` is set to the default value; you can remove it from your config",
                    KEY
                );
            }
            raw.template_pack.as_deref().unwrap_or_else(|| {
                log::info!(
                    "`{}.template-pack` not set; defaulting to {}",
                    KEY,
                    DEFAULT_TEMPLATE_PACK
                );
                DEFAULT_TEMPLATE_PACK
            })
        };
        #[cfg(not(feature = "brainium"))]
        let template_pack = &raw.template_pack;

        let template_pack = Pack::lookup(template_pack).map_err(Error::TemplatePackNotFound)?;

        Ok(Self {
            root_dir,
            name,
            stylized_name,
            domain,
            asset_dir,
            template_pack,
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
        self.root_dir().join(&self.asset_dir)
    }

    pub fn template_pack(&self) -> &Pack {
        &self.template_pack
    }
}
