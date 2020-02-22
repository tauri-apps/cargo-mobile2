mod detect;
mod raw;

pub use self::{detect::*, raw::*};

use super::app_name;
use crate::util;
use heck::SnekCase as _;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};

static DEFAULT_APP_ROOT: &'static str = ".";

#[derive(Debug)]
pub enum AppRootInvalid {
    NormalizationFailed {
        app_root: String,
        cause: util::NormalizationError,
    },
    OutsideOfProject {
        app_root: String,
        project_root: PathBuf,
    },
}

impl Display for AppRootInvalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalizationFailed { app_root, cause } => {
                write!(f, "{:?} couldn't be normalized: {}", app_root, cause)
            }
            Self::OutsideOfProject {
                app_root,
                project_root,
            } => write!(
                f,
                "{:?} is outside of the project root ({:?}).",
                app_root, project_root,
            ),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    AppNameInvalid(app_name::Invalid),
    DomainInvalid { domain: String },
    AppRootInvalid(AppRootInvalid),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppNameInvalid(err) => write!(f, "`ginit.app-name` invalid: {}", err),
            Self::DomainInvalid { domain } => write!(
                f,
                "`ginit.domain` invalid: {:?} isn't valid domain syntax.",
                domain
            ),
            Self::AppRootInvalid(err) => write!(f, "`ginit.app-root` invalid: {}", err),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Shared {
    project_root: PathBuf,
    app_name: String,
    stylized_app_name: Option<String>,
    domain: String,
    app_root: String,
    plugins: Vec<String>,
}

impl Shared {
    pub fn from_raw(project_root: PathBuf, raw_config: Raw) -> Result<Self, Error> {
        let app_name = app_name::validate(raw_config.app_name).map_err(Error::AppNameInvalid)?;
        let stylized_app_name = raw_config.stylized_app_name;
        let domain = {
            let domain = raw_config.domain;
            if publicsuffix::Domain::has_valid_syntax(&domain) {
                Ok(domain)
            } else {
                Err(Error::DomainInvalid { domain })
            }
        }?;
        let app_root = raw_config.app_root.map(|app_root| {
            if app_root.as_str() == DEFAULT_APP_ROOT {
                log::warn!("`ginit.app-root` is set to the default value; you can remove it from your config");
            }
            if util::normalize_path(&app_root).map_err(|cause| Error::AppRootInvalid(AppRootInvalid::NormalizationFailed {
                app_root: app_root.clone(),
                cause,
            }))?.starts_with(&project_root) {
                Ok(app_root)
            } else {
                Err(Error::AppRootInvalid(AppRootInvalid::OutsideOfProject { app_root, project_root: project_root.clone() }))
            }
        })
        .unwrap_or_else(|| {
            log::info!(
                "`ginit.app-root` not set; defaulting to {:?}",
                DEFAULT_APP_ROOT
            );
            Ok(DEFAULT_APP_ROOT.to_owned())
        })?;
        let plugins = raw_config.plugins.unwrap_or_default();
        Ok(Self {
            project_root,
            app_name,
            stylized_app_name,
            domain,
            app_root,
            plugins,
        })
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn prefix_path(&self, path: impl AsRef<Path>) -> PathBuf {
        util::prefix_path(self.project_root(), path)
    }

    pub fn unprefix_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PathBuf, util::UnprefixPathError> {
        util::unprefix_path(self.project_root(), path)
    }

    pub fn app_name(&self) -> &str {
        &self.app_name
    }

    pub fn app_name_snake(&self) -> String {
        self.app_name().to_snek_case()
    }

    pub fn stylized_app_name(&self) -> &str {
        self.stylized_app_name
            .as_ref()
            .unwrap_or_else(|| &self.app_name)
    }

    pub fn reverse_domain(&self) -> String {
        self.domain
            .clone()
            .split('.')
            .rev()
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn app_root(&self) -> PathBuf {
        self.prefix_path(&self.app_root)
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.app_root().join("Cargo.toml")
    }

    pub fn asset_path(&self) -> PathBuf {
        self.app_root().join("res")
    }

    pub fn plugins(&self) -> &[String] {
        &self.plugins
    }

    pub(crate) fn insert_template_data(&self, map: &mut bicycle::JsonMap) {
        map.insert("shared", &self);
        map.insert("app-name", self.app_name());
        map.insert("app-name-snake", self.app_name_snake());
        map.insert("stylized-app-name", self.stylized_app_name());
        map.insert("reverse-domain", self.reverse_domain());
    }
}
