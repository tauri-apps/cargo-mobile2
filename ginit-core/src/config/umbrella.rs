use super::{
    shared::{self, Shared},
    ConfigTrait,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Error {
    DiscoverFailed(io::Error),
    ReadFailed {
        path: PathBuf,
        cause: io::Error,
    },
    ParseFailed {
        path: PathBuf,
        cause: toml::de::Error,
    },
    SharedConfigInvalid {
        path: PathBuf,
        cause: shared::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DiscoverFailed(err) => write!(
                f,
                "Failed to canonicalize path while searching for project root: {}",
                err
            ),
            Self::ReadFailed { path, cause } => {
                write!(f, "Failed to read config file at {:?}: {}", path, cause)
            }
            Self::ParseFailed { path, cause } => {
                write!(f, "Failed to parse config file at {:?}: {}", path, cause)
            }
            Self::SharedConfigInvalid { path, cause } => {
                write!(f, "`ginit` config invalid in {:?}: {}", path, cause)
            }
        }
    }
}

#[derive(Debug)]
pub enum PluginErrorCause<C: ConfigTrait> {
    ParseFailed(toml::de::Error),
    ConfigInvalid(C::Error),
}

#[derive(Debug)]
pub struct PluginError<C: ConfigTrait> {
    name: String,
    cause: PluginErrorCause<C>,
}

impl<C: ConfigTrait> Display for PluginError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            PluginErrorCause::ParseFailed(err) => {
                write!(f, "Failed to parse `{}` config section: {}", self.name, err)
            }
            PluginErrorCause::ConfigInvalid(err) => {
                write!(f, "`{}` config invalid: {}", self.name, err)
            }
        }
    }
}

impl<C: ConfigTrait> PluginError<C> {
    pub fn new(name: impl Into<String>, cause: PluginErrorCause<C>) -> Self {
        Self {
            name: name.into(),
            cause,
        }
    }
}

#[derive(Debug)]
pub enum LoadOrPluginError<C: ConfigTrait> {
    LoadFailed(Error),
    ConfigFileMissing,
    PluginFailed(PluginError<C>),
}

impl<C: ConfigTrait> Display for LoadOrPluginError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadFailed(err) => write!(f, "{}", err),
            Self::ConfigFileMissing => write!(f, "Failed to find {}!", Umbrella::file_name()),
            Self::PluginFailed(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Umbrella {
    shared: Shared,
    #[serde(flatten)]
    plugins: HashMap<String, toml::Value>,
}

impl Umbrella {
    pub fn file_name() -> String {
        format!("{}.toml", crate::NAME)
    }

    pub fn discover_root(cwd: impl AsRef<Path>) -> io::Result<Option<PathBuf>> {
        let file_name = Self::file_name();
        let mut path = cwd.as_ref().canonicalize()?.join(&file_name);
        // TODO: fold
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

    pub fn load(cwd: impl AsRef<Path>) -> Result<Option<Self>, Error> {
        #[derive(Debug, Deserialize, Serialize)]
        struct Raw {
            #[serde(rename = "ginit")]
            shared: shared::Raw,
            #[serde(flatten)]
            plugins: HashMap<String, toml::Value>,
        }
        if let Some(project_root) = Self::discover_root(cwd).map_err(Error::DiscoverFailed)? {
            let path = project_root.join(&Self::file_name());
            let bytes = fs::read(&path).map_err(|cause| Error::ReadFailed {
                path: path.clone(),
                cause,
            })?;
            let raw = toml::from_slice::<Raw>(&bytes).map_err(|cause| Error::ParseFailed {
                path: path.clone(),
                cause,
            })?;
            Ok(Some(Self {
                shared: Shared::from_raw(project_root, raw.shared)
                    .map_err(|cause| Error::SharedConfigInvalid { path, cause })?
                    .into(),
                plugins: raw.plugins,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn shared(&self) -> &Shared {
        &self.shared
    }

    fn extract_plugin<C: ConfigTrait>(mut self, name: &str) -> Result<C, PluginError<C>> {
        C::from_raw(
            self.shared,
            self.plugins
                .remove(name)
                .map(|plugin| plugin.try_into())
                .transpose()
                .map_err(|err| PluginError::new(name, PluginErrorCause::ParseFailed(err)))?,
        )
        .map_err(|err| PluginError::new(name, PluginErrorCause::ConfigInvalid(err)))
    }

    pub fn load_plugin<C: ConfigTrait>(name: &str) -> Result<Option<C>, LoadOrPluginError<C>> {
        Self::load(".")
            .map_err(LoadOrPluginError::LoadFailed)?
            .map(|umbrella| {
                umbrella
                    .extract_plugin(name)
                    .map_err(LoadOrPluginError::PluginFailed)
            })
            .transpose()
    }
}
