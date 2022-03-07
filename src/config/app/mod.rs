mod common_email_providers;
pub mod domain;
pub mod name;
mod raw;

pub use self::raw::*;

use crate::{
    templating::{self, Pack},
    util::{self, cli::Report},
};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub static KEY: &str = "app";

pub static DEFAULT_ASSET_DIR: &str = "assets";
pub static IMPLIED_TEMPLATE_PACK: &str = "brainstorm";
pub static DEFAULT_TEMPLATE_PACK: &str = if cfg!(feature = "brainium") {
    IMPLIED_TEMPLATE_PACK
} else {
    "bevy"
};

#[derive(Debug)]
pub enum Error {
    NameInvalid(name::Invalid),
    DomainInvalid {
        domain: String,
        cause: domain::DomainError,
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
            Self::DomainInvalid { domain, cause } => Report::error(
                msg,
                format!("`{}.domain` {:?} isn't valid: {}", KEY, domain, cause),
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct App {
    root_dir: PathBuf,
    name: String,
    stylized_name: String,
    domain: String,
    asset_dir: PathBuf,
    #[serde(skip)]
    template_pack: Pack,
}

impl App {
    pub fn from_raw(root_dir: PathBuf, raw: Raw) -> Result<Self, Error> {
        assert!(root_dir.is_absolute(), "root must be absolute");

        let name = name::validate(raw.name).map_err(Error::NameInvalid)?;

        let stylized_name = raw.stylized_name.unwrap_or_else(|| name.clone());

        let domain = {
            let domain = raw.domain;
            domain::check_domain_syntax(&domain)
                .map_err(|cause| Error::DomainInvalid {
                    domain: domain.clone(),
                    cause,
                })
                .map(|()| domain)
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

        let template_pack = {
            if raw.template_pack.as_deref() == Some(IMPLIED_TEMPLATE_PACK) {
                log::warn!(
                    "`{}.template-pack` is set to the implied value; you can remove it from your config",
                    KEY
                );
            }
            raw.template_pack.as_deref().unwrap_or_else(|| {
                log::info!(
                    "`{}.template-pack` not set; defaulting to {}",
                    KEY,
                    IMPLIED_TEMPLATE_PACK
                );
                IMPLIED_TEMPLATE_PACK
            })
        };
        let template_pack = Pack::lookup_app(template_pack).map_err(Error::TemplatePackNotFound)?;

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
        use heck::ToSnekCase as _;
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
