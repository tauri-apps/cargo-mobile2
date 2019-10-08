use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Error {
    DiscoverFailed(io::Error),
    OpenFailed(io::Error),
    ReadFailed(io::Error),
    ParseFailed(toml::de::Error),
    SharedConfigInvalid(ginit_core::config::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DiscoverFailed(err) => write!(
                f,
                "Failed to canonicalize path while searching for project root: {}",
                err
            ),
            Self::OpenFailed(err) => write!(f, "Failed to open config file: {}", err),
            Self::ReadFailed(err) => write!(f, "Failed to read config file: {}", err),
            Self::ParseFailed(err) => write!(f, "Failed to parse config file: {}", err),
            Self::SharedConfigInvalid(err) => write!(f, "`ginit` config invalid: {}", err),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Umbrella {
    shared: ginit_core::config::Config,
    #[serde(flatten)]
    plugins: HashMap<String, toml::Value>,
}

impl Umbrella {
    pub fn file_name() -> String {
        format!("{}.toml", ginit_core::NAME)
    }

    fn discover_root(cwd: impl AsRef<Path>) -> io::Result<Option<PathBuf>> {
        let file_name = Self::file_name();
        let mut path = cwd.as_ref().canonicalize()?.join(&file_name);
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
            ginit: ginit_core::config::Raw,
            #[serde(flatten)]
            plugins: HashMap<String, toml::Value>,
        }
        if let Some(project_root) = Self::discover_root(cwd).map_err(Error::DiscoverFailed)? {
            let path = project_root.join(&Self::file_name());
            let mut file = File::open(&path).map_err(Error::OpenFailed)?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).map_err(Error::ReadFailed)?;
            let raw = toml::from_slice::<Raw>(&contents).map_err(Error::ParseFailed)?;
            Ok(Some(Self {
                shared: ginit_core::config::Config::from_raw(project_root, raw.ginit)
                    .map_err(Error::SharedConfigInvalid)?,
                plugins: raw.plugins,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn shared(&self) -> &ginit_core::config::Config {
        &self.shared
    }

    pub fn plugin(&self, plugin_name: &str) -> Option<Vec<u8>> {
        self.plugins.get(plugin_name).map(|value| {
            toml::to_vec(&value)
                .expect("Couldn't serialize the TOML data we deserialized, which is really weird")
        })
    }
}
