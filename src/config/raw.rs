use super::{app, TemplatePack};
use crate::{
    android, apple,
    util::{cli::TextWrapper, submodule::Submodule},
};
use serde::{Deserialize, Serialize};

use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum PromptError {
    AppFailed(app::PromptError),
    AppleFailed(apple::config::PromptError),
}

impl Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppFailed(err) => {
                write!(f, "Failed to prompt for `{}` config: {}", app::KEY, err)
            }
            Self::AppleFailed(err) => {
                write!(f, "Failed to prompt for `{}` config: {}", apple::NAME, err)
            }
        }
    }
}

#[derive(Debug)]
pub enum DetectError {
    AppFailed(app::DetectError),
    AppleFailed(apple::config::DetectError),
}

impl Display for DetectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppFailed(err) => write!(f, "Failed to detect `{}` config: {}", app::KEY, err),
            Self::AppleFailed(err) => {
                write!(f, "Failed to detect `{}` config: {}", apple::NAME, err)
            }
        }
    }
}

fn default_template_packs_and_submodules() -> (Option<Vec<TemplatePack>>, Option<Vec<Submodule>>) {
    let brainium = false;
    if brainium {
        (
            Some(vec![TemplatePack::with_src(
                "~/.cargo-mobile/templates/rust-lib-app",
            )]),
            Some(vec![Submodule::with_remote_and_path(
                "git@bitbucket.org:brainium/rust_lib.git",
                "rust-lib",
            )]),
        )
    } else {
        (
            Some(vec![TemplatePack::with_src(
                "~/.cargo-mobile/templates/wgpu-app",
            )]),
            None,
        )
    }
}

#[derive(Debug)]
pub enum LoadError {
    DiscoverFailed(io::Error),
    ReadFailed {
        path: PathBuf,
        cause: io::Error,
    },
    ParseFailed {
        path: PathBuf,
        cause: toml::de::Error,
    },
}

impl Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DiscoverFailed(err) => write!(
                f,
                "Failed to canonicalize path while searching for config file: {}",
                err
            ),
            Self::ReadFailed { path, cause } => {
                write!(f, "Failed to read config file at {:?}: {}", path, cause)
            }
            Self::ParseFailed { path, cause } => {
                write!(f, "Failed to parse config file at {:?}: {}", path, cause)
            }
        }
    }
}

#[derive(Debug)]
pub enum WriteError {
    SerializeFailed(toml::ser::Error),
    WriteFailed(io::Error),
}

impl Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SerializeFailed(err) => write!(f, "Failed to serialize config: {}", err),
            Self::WriteFailed(err) => write!(f, "Failed to write config: {}", err),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    pub app: app::Raw,
    pub template_packs: Option<Vec<TemplatePack>>,
    pub submodules: Option<Vec<Submodule>>,
    pub android: Option<android::config::Raw>,
    pub apple: Option<apple::config::Raw>,
}

impl Raw {
    pub fn prompt(wrapper: &TextWrapper) -> Result<Self, PromptError> {
        let app = app::Raw::prompt(wrapper).map_err(PromptError::AppFailed)?;
        let (template_packs, submodules) = default_template_packs_and_submodules();
        let apple = apple::config::Raw::prompt(wrapper).map_err(PromptError::AppleFailed)?;
        Ok(Self {
            app,
            template_packs,
            submodules,
            android: None,
            apple: Some(apple),
        })
    }

    pub fn detect() -> Result<Self, DetectError> {
        let app = app::Raw::detect().map_err(DetectError::AppFailed)?;
        let (template_packs, submodules) = default_template_packs_and_submodules();
        let apple = apple::config::Raw::detect().map_err(DetectError::AppleFailed)?;
        Ok(Self {
            app,
            template_packs,
            submodules,
            android: None,
            apple: Some(apple),
        })
    }

    pub fn file_name() -> String {
        format!("{}.toml", crate::NAME)
    }

    pub fn discover_root(cwd: impl AsRef<Path>) -> io::Result<Option<PathBuf>> {
        let file_name = Self::file_name();
        let mut path = cwd.as_ref().canonicalize()?.join(&file_name);
        log::info!("looking for config file at {:?}", path);
        while !path.exists() {
            if let Some(parent) = path.parent().and_then(Path::parent) {
                path = parent.join(&file_name);
                log::info!("looking for config file at {:?}", path);
            } else {
                log::info!("no config file was ever found");
                return Ok(None);
            }
        }
        log::info!("found config file at {:?}", path);
        path.pop();
        Ok(Some(path))
    }

    pub fn load(cwd: impl AsRef<Path>) -> Result<Option<(PathBuf, Self)>, LoadError> {
        Self::discover_root(cwd)
            .map_err(LoadError::DiscoverFailed)?
            .map(|root_dir| {
                let path = root_dir.join(Self::file_name());
                let bytes = fs::read(&path).map_err(|cause| LoadError::ReadFailed {
                    path: path.clone(),
                    cause,
                })?;
                toml::from_slice::<Self>(&bytes)
                    .map(|raw| (root_dir, raw))
                    .map_err(|cause| LoadError::ParseFailed {
                        path: path.clone(),
                        cause,
                    })
            })
            .transpose()
    }

    pub fn write(&self, root_dir: &Path) -> Result<(), WriteError> {
        let bytes = toml::to_vec(self).map_err(WriteError::SerializeFailed)?;
        let path = root_dir.join(Self::file_name());
        log::info!("writing config to {:?}", path);
        fs::write(path, bytes).map_err(WriteError::WriteFailed)
    }
}
