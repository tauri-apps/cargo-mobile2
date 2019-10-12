mod map;

pub use map::Map;

use ginit_core::{
    exports::into_result::{command::CommandError, IntoResult as _},
    opts, Manifest,
};
use std::{
    ffi::OsStr,
    fmt::{self, Display},
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command},
};

#[derive(Debug)]
pub enum NewError {
    BinMissing {
        tried: PathBuf,
    },
    ManifestMissing {
        tried: PathBuf,
    },
    ManifestOpenFailed {
        tried: PathBuf,
        cause: io::Error,
    },
    ManifestReadFailed {
        tried: PathBuf,
        cause: io::Error,
    },
    ManifestParseFailed {
        tried: PathBuf,
        cause: toml::de::Error,
    },
    NameMismatch {
        found: String,
    },
}

impl Display for NewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BinMissing { tried } => write!(f, "Binary not found: tried {:?}", tried),
            Self::ManifestMissing { tried } => write!(f, "Manifest not found: tried {:?}", tried),
            Self::ManifestOpenFailed { tried, cause } => {
                write!(f, "Failed to open manifest at {:?}: {}", tried, cause)
            }
            Self::ManifestReadFailed { tried, cause } => {
                write!(f, "Failed to read manifest at {:?}: {}", tried, cause)
            }
            Self::ManifestParseFailed { tried, cause } => {
                write!(f, "Failed to parse manifest at {:?}: {}", tried, cause)
            }
            Self::NameMismatch { found } => write!(
                f,
                "Manifest says the plugin's name is actually {:?}, and must make up its mind.",
                found
            ),
        }
    }
}

#[derive(Debug)]
pub enum RunError {
    SpawnFailed(CommandError),
}

impl Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpawnFailed(err) => write!(f, "Failed to spawn process: {}", err),
        }
    }
}

#[derive(Debug)]
pub struct ProcHandle {
    inner: Child,
}

impl Drop for ProcHandle {
    fn drop(&mut self) {
        if let Err(err) = self.inner.wait() {
            eprintln!("{}", err);
        }
    }
}

impl From<Child> for ProcHandle {
    fn from(inner: Child) -> Self {
        Self { inner }
    }
}

#[derive(Debug)]
pub struct Plugin {
    bin_path: PathBuf,
    manifest_path: PathBuf,
    manifest: Manifest,
}

impl Plugin {
    pub fn new(name: impl AsRef<str>) -> Result<Self, NewError> {
        let name = name.as_ref();
        let bin_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(&format!("../target/debug/ginit-{}", name));
        if !bin_path.is_file() {
            return Err(NewError::BinMissing { tried: bin_path });
        }
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(&format!("../plugins/ginit-{}/plugin.toml", name));
        if !manifest_path.is_file() {
            return Err(NewError::ManifestMissing {
                tried: manifest_path,
            });
        }
        let manifest = {
            let mut manifest_file =
                File::open(&manifest_path).map_err(|cause| NewError::ManifestOpenFailed {
                    tried: manifest_path.clone(),
                    cause,
                })?;
            let mut manifest_raw = Vec::new();
            manifest_file
                .read_to_end(&mut manifest_raw)
                .map_err(|cause| NewError::ManifestReadFailed {
                    tried: manifest_path.clone(),
                    cause,
                })?;
            toml::from_slice(&manifest_raw).map_err(|cause| NewError::ManifestParseFailed {
                tried: manifest_path.clone(),
                cause,
            })?
        };
        Ok(Self {
            manifest,
            bin_path,
            manifest_path,
        })
    }

    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    pub fn description(&self) -> &str {
        &self.manifest.description
    }

    pub fn supports(&self, feature: impl AsRef<str>) -> bool {
        self.manifest
            .supports
            .iter()
            .any(|supported| supported.as_str() == feature.as_ref())
    }

    pub fn run(
        &self,
        noise_level: opts::NoiseLevel,
        interactivity: opts::Interactivity,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<ProcHandle, RunError> {
        let mut command = Command::new(&self.bin_path);
        match noise_level {
            opts::NoiseLevel::Polite => (),
            opts::NoiseLevel::LoudAndProud => {
                command.arg("-v");
            }
            opts::NoiseLevel::FranklyQuitePedantic => {
                command.arg("-vv");
            }
        }
        match interactivity {
            opts::Interactivity::Full => (),
            opts::Interactivity::None => {
                command.arg("--non-interactive");
            }
        }
        command.args(args);
        command
            .spawn()
            .into_result()
            .map_err(RunError::SpawnFailed)
            .map(Into::into)
    }
}
