mod manifest;
mod map;

pub use map::Map;

use self::manifest::Manifest;
use ginit_core::{
    exports::into_result::{command::CommandError, IntoResult as _},
    opts,
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
    BinMissing { tried: PathBuf },
    ManifestFailed(manifest::Error),
}

impl Display for NewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BinMissing { tried } => write!(f, "Binary not found: tried {:?}", tried),
            Self::ManifestFailed(err) => write!(f, "{}", err),
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
    manifest: Manifest,
}

impl Plugin {
    pub fn new(name: impl AsRef<str>) -> Result<Self, NewError> {
        let name = name.as_ref();
        let manifest = Manifest::load(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(&format!("../plugins/ginit-{}/Cargo.toml", name)),
        )
        .map_err(NewError::ManifestFailed)?;
        let bin_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(&format!("../target/debug/ginit-{}", name));
        if !bin_path.is_file() {
            return Err(NewError::BinMissing { tried: bin_path });
        }
        Ok(Self { manifest, bin_path })
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
