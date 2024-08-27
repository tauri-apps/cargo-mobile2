mod common_email_providers;
pub mod identifier;
pub mod lib_name;
pub mod name;
mod raw;

pub use self::raw::*;

use crate::{
    opts::Profile,
    templating::{self, Pack},
    util::{self, cli::Report},
};
use serde::Serialize;
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

pub static KEY: &str = "app";

pub static DEFAULT_ASSET_DIR: &str = "assets";
pub static IMPLIED_TEMPLATE_PACK: &str = "brainstorm";
pub static DEFAULT_TEMPLATE_PACK: &str = if cfg!(feature = "brainium") {
    IMPLIED_TEMPLATE_PACK
} else {
    "bevy"
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("app.name invalid: {0}")]
    NameInvalid(name::Invalid),
    #[error("app.lib_name invalid: {0}")]
    LibNameInvalid(lib_name::Invalid),
    #[error("`app.identifier` {identifier} isn't valid: {cause}")]
    IdentifierInvalid {
        identifier: String,
        cause: identifier::IdentifierError,
    },
    #[error("`app.asset-dir` {asset_dir} couldn't be normalized: {cause}")]
    AssetDirNormalizationFailed {
        asset_dir: PathBuf,
        cause: util::NormalizationError,
    },
    #[error("`app.asset-dir` {asset_dir} is outside of the app root {root_dir}")]
    AssetDirOutsideOfAppRoot {
        asset_dir: PathBuf,
        root_dir: PathBuf,
    },
    #[error(transparent)]
    TemplatePackNotFound(templating::LookupError),
}

impl Error {
    pub fn report(&self, msg: &str) -> Report {
        Report::error(msg, self)
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct App {
    root_dir: PathBuf,
    name: String,
    lib_name: Option<String>,
    stylized_name: String,
    identifier: String,
    asset_dir: PathBuf,
    #[serde(skip)]
    template_pack: Pack,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    target_dir_resolver: Option<Arc<Box<dyn Fn(&str, Profile) -> PathBuf>>>,
}

impl Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("root_dir", &self.root_dir)
            .field("name", &self.name)
            .field("stylized_name", &self.stylized_name)
            .field("identifier", &self.identifier)
            .field("asset_dir", &self.asset_dir)
            .field("template_pack", &self.template_pack)
            .finish()
    }
}

impl App {
    pub fn from_raw(root_dir: PathBuf, raw: Raw) -> Result<Self, Error> {
        assert!(root_dir.is_absolute(), "root must be absolute");

        let name = name::validate(raw.name).map_err(Error::NameInvalid)?;

        let lib_name = raw
            .lib_name
            .map(|n| lib_name::validate(n).map_err(Error::LibNameInvalid))
            .transpose()?;

        let stylized_name = raw.stylized_name.unwrap_or_else(|| name.clone());

        let identifier = {
            let identifier = raw.identifier;
            identifier::check_identifier_syntax(&identifier)
                .map_err(|cause| Error::IdentifierInvalid {
                    identifier: identifier.clone(),
                    cause,
                })
                .map(|()| identifier)
        }?;

        if raw.asset_dir.as_deref() == Some(DEFAULT_ASSET_DIR) {
            log::warn!(
                "`{}.asset-dir` is set to the default value; you can remove it from your config",
                KEY
            );
        }
        let asset_dir = raw
            .asset_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| DEFAULT_ASSET_DIR.into());
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
            raw.template_pack
                .as_deref()
                .unwrap_or(IMPLIED_TEMPLATE_PACK)
        };
        let template_pack = if cfg!(feature = "cli") {
            Pack::lookup_app(template_pack).map_err(Error::TemplatePackNotFound)?
        } else {
            Pack::Simple(Default::default())
        };

        Ok(Self {
            root_dir,
            name,
            lib_name,
            stylized_name,
            identifier,
            asset_dir,
            template_pack,
            target_dir_resolver: None,
        })
    }

    pub fn with_target_dir_resolver<F: Fn(&str, Profile) -> PathBuf + 'static>(
        mut self,
        resolver: F,
    ) -> Self {
        self.target_dir_resolver
            .replace(Arc::new(Box::new(resolver)));
        self
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn target_dir(&self, triple: &str, profile: Profile) -> PathBuf {
        if let Some(resolver) = &self.target_dir_resolver {
            resolver(triple, profile)
        } else if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
            self.prefix_path(format!("{}/{}/{}", target, triple, profile.as_str()))
        } else if let Ok(target) = std::env::var("CARGO_BUILD_TARGET_DIR") {
            self.prefix_path(format!("{}/{}/{}", target, triple, profile.as_str()))
        } else {
            self.prefix_path(format!("target/{}/{}", triple, profile.as_str()))
        }
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

    pub fn lib_name(&self) -> String {
        self.lib_name.clone().unwrap_or_else(|| self.name_snake())
    }

    pub fn stylized_name(&self) -> &str {
        &self.stylized_name
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn android_identifier_escape_kotlin_keyword(&self) -> String {
        self.identifier
            .split('.')
            .map(|s| {
                if crate::reserved_names::KOTLIN_ONLY_KEYWORDS.contains(&s) {
                    format!("`{}`", s)
                } else {
                    s.to_string()
                }
            })
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
