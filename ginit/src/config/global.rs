use crate::app_name;
use serde::{Deserialize, Serialize};
use std::{
    fmt, io,
    path::{Path, PathBuf},
};

static DEFAULT_APP_ROOT: &'static str = ".";

#[derive(Debug)]
pub enum AppRootInvalid {
    CanonicalizationFailed {
        app_root: PathBuf,
        cause: io::Error,
    },
    OutsideOfProject {
        app_root: PathBuf,
        project_root: PathBuf,
    },
}

impl fmt::Display for AppRootInvalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CanonicalizationFailed { app_root, cause } => {
                write!(f, "{:?} couldn't be canonicalized: {}", app_root, cause)
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
pub enum ValidationError {
    AppNameInvalid(app_name::Invalid),
    DomainInvalid { domain: String },
    AppRootInvalid(AppRootInvalid),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppNameInvalid(err) => write!(f, "`global.app-name` invalid: {}", err),
            Self::DomainInvalid { domain } => write!(
                f,
                "`global.domain` invalid: {:?} isn't valid domain syntax.",
                domain
            ),
            Self::AppRootInvalid(err) => write!(f, "`global.app-root` invalid: {}", err),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RawGlobalConfig {
    #[serde(alias = "app-name")]
    app_name: String,
    #[serde(alias = "stylized-app-name")]
    stylized_app_name: Option<String>,
    domain: String,
    #[serde(alias = "app-root")]
    app_root: Option<String>,
    // These aren't used anymore, and only kept in so we can emit warnings about them!
    source_root: Option<String>,
    manifest_path: Option<String>,
    asset_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct GlobalConfig {
    pub(super) app_name: String,
    pub(super) stylized_app_name: Option<String>,
    pub(super) domain: String,
    pub(super) app_root: String,
}

impl GlobalConfig {
    pub(super) fn from_raw(
        project_root: &Path,
        raw_config: RawGlobalConfig,
    ) -> Result<Self, ValidationError> {
        if raw_config.source_root.is_some() {
            log::warn!("`global.source_root` specified in {}.toml - this config key is no longer needed, and will be ignored", crate::NAME);
        }
        if raw_config.manifest_path.is_some() {
            log::warn!("`global.manifest_path` specified in {}.toml - this config key is no longer needed, and will be ignored", crate::NAME);
        }
        if raw_config.asset_path.is_some() {
            log::warn!("`global.asset_path` specified in {}.toml - this config key is no longer needed, and will be ignored", crate::NAME);
        }
        Ok(Self {
            app_name: app_name::validate(raw_config.app_name).map_err(ValidationError::AppNameInvalid)?,
            stylized_app_name: raw_config.stylized_app_name,
            domain: {
                let domain = raw_config.domain;
                if publicsuffix::Domain::has_valid_syntax(&domain) {
                    Ok(domain)
                } else {
                    Err(ValidationError::DomainInvalid { domain })
                }
            }?,
            app_root: raw_config.app_root.map(|app_root| {
                if app_root.as_str() == DEFAULT_APP_ROOT {
                    log::warn!("`global.app-root` is set to the default value; you can remove it from your config");
                }
                if Path::new(&app_root).canonicalize().map_err(|cause| ValidationError::AppRootInvalid(AppRootInvalid::CanonicalizationFailed {
                    app_root: app_root.clone().into(),
                    cause,
                }))?.starts_with(project_root) {
                    Ok(app_root)
                } else {
                    Err(ValidationError::AppRootInvalid(AppRootInvalid::OutsideOfProject { app_root: app_root.into(), project_root: project_root.to_owned() }))
                }
            })
            .unwrap_or_else(|| {
                log::info!(
                    "`global.app-root` not set; defaulting to {}",
                    DEFAULT_APP_ROOT
                );
                Ok(DEFAULT_APP_ROOT.to_owned())
            })?,
        })
    }
}
