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
    io,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
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
    WaitFailed(io::Error),
    CommandFailed(CommandError),
}

impl Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpawnFailed(err) => write!(f, "Failed to spawn process: {}", err),
            Self::WaitFailed(err) => write!(f, "Failed to wait for exit status: {}", err),
            Self::CommandFailed(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug)]
#[must_use = "proc handles must be `wait`ed on, or they won't stop"]
pub struct ProcHandle {
    inner: Option<Child>,
}

impl Drop for ProcHandle {
    fn drop(&mut self) {
        if self.inner.is_some() {
            log::error!("proc handle dropped without being waited on");
        }
    }
}

impl From<Child> for ProcHandle {
    fn from(inner: Child) -> Self {
        Self { inner: Some(inner) }
    }
}

impl ProcHandle {
    pub fn wait(mut self) -> io::Result<ExitStatus> {
        self.inner
            .take()
            .expect("developer error: `ProcHandle` vacant")
            .wait()
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

    fn run(
        &self,
        noise_level: opts::NoiseLevel,
        interactivity: opts::Interactivity,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<ProcHandle, RunError> {
        let mut command = {
            let mut command = Command::new(&self.bin_path);
            if let Ok(backtrace) = std::env::var("RUST_BACKTRACE") {
                command.env("RUST_BACKTRACE", backtrace);
            }
            if let Ok(log) = std::env::var("RUST_LOG") {
                command.env("RUST_LOG", log);
            }
            command.stdin(Stdio::piped());
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
        };
        let handle = command
            .spawn()
            .into_result()
            .map_err(RunError::SpawnFailed)?;
        Ok(handle.into())
    }

    pub fn run_and_wait(
        &self,
        noise_level: opts::NoiseLevel,
        interactivity: opts::Interactivity,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<(), RunError> {
        self.run(noise_level, interactivity, args)?
            .wait()
            .map_err(RunError::WaitFailed)?
            .into_result()
            .map_err(RunError::CommandFailed)
    }
}
