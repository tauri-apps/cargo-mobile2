mod map;

pub use map::Map;

use ginit_core::{
    bundle::manifest::{self, Manifest},
    exports::into_result::{command::CommandError, IntoResult as _},
    opts,
};
use std::{
    ffi::OsStr,
    fmt::{self, Display},
    io,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
};

#[derive(Debug)]
pub enum LoadErrorCause {
    BinMissing { tried: PathBuf },
    ManifestFailed(manifest::Error),
}

#[derive(Debug)]
pub struct LoadError {
    plugin_name: String,
    cause: LoadErrorCause,
}

impl Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            LoadErrorCause::BinMissing { tried } => write!(
                f,
                "Binary not found for plugin {:?}: tried {:?}",
                self.plugin_name, tried
            ),
            LoadErrorCause::ManifestFailed(err) => write!(
                f,
                "Failed to load manifest for plugin {:?}: {}",
                self.plugin_name, err
            ),
        }
    }
}

#[derive(Debug)]
pub enum RunErrorCause {
    SpawnFailed(CommandError),
    WaitFailed(io::Error),
    CommandFailed(CommandError),
}

#[derive(Debug)]
pub struct RunError {
    plugin_name: String,
    cause: RunErrorCause,
}

impl Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            RunErrorCause::SpawnFailed(err) => write!(
                f,
                "Failed to spawn process for plugin {:?}: {}",
                self.plugin_name, err
            ),
            RunErrorCause::WaitFailed(err) => write!(
                f,
                "Failed to wait for exit status for plugin {:?}: {}",
                self.plugin_name, err
            ),
            RunErrorCause::CommandFailed(err) => {
                write!(f, "Plugin {:?} failed: {}", self.plugin_name, err)
            }
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
    short_name: String,
}

impl Plugin {
    pub fn new(name: impl AsRef<str>) -> Result<Self, LoadError> {
        let name = name.as_ref();
        let manifest = Manifest::load_from_cargo_toml(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(&format!("../plugins/ginit-{}/Cargo.toml", name)),
        )
        .map_err(|cause| LoadError {
            plugin_name: name.to_owned(),
            cause: LoadErrorCause::ManifestFailed(cause),
        })?;
        let bin_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(&format!("../target/debug/ginit-{}", name));
        if !bin_path.is_file() {
            return Err(LoadError {
                plugin_name: name.to_owned(),
                cause: LoadErrorCause::BinMissing { tried: bin_path },
            });
        }
        let short_name = manifest.short_name();
        Ok(Self {
            manifest,
            bin_path,
            short_name,
        })
    }

    pub fn name(&self) -> &str {
        &self.short_name
    }

    pub fn description(&self) -> &str {
        self.manifest.description()
    }

    fn command(
        &self,
        noise_level: opts::NoiseLevel,
        interactivity: opts::Interactivity,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Command {
        let mut command = Command::new(&self.bin_path);
        if let Ok(backtrace) = std::env::var("RUST_BACKTRACE") {
            command.env("RUST_BACKTRACE", backtrace);
        }
        if let Ok(log) = std::env::var("RUST_LOG") {
            command.env("RUST_LOG", log);
        }
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
    }

    pub fn run(
        &self,
        noise_level: opts::NoiseLevel,
        interactivity: opts::Interactivity,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<ProcHandle, RunError> {
        self.command(noise_level, interactivity, args)
            .spawn()
            .into_result()
            .map(ProcHandle::from)
            .map_err(|cause| RunError {
                plugin_name: self.name().to_owned(),
                cause: RunErrorCause::SpawnFailed(cause),
            })
    }

    pub fn run_and_wait(
        &self,
        noise_level: opts::NoiseLevel,
        interactivity: opts::Interactivity,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<(), RunError> {
        self.run(noise_level, interactivity, args)?
            .wait()
            .map_err(|cause| RunError {
                plugin_name: self.name().to_owned(),
                cause: RunErrorCause::WaitFailed(cause),
            })?
            .into_result()
            .map_err(|cause| RunError {
                plugin_name: self.name().to_owned(),
                cause: RunErrorCause::CommandFailed(cause),
            })
    }
}
