mod map;

pub use map::Map;

use ginit_core::{
    exports::bossy,
    opts,
    storage::manifest::{self, Manifest},
};
use std::{
    ffi::OsStr,
    fmt::{self, Display},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Cause {
    BinMissing { tried: PathBuf },
    ManifestFailed(manifest::Error),
}

#[derive(Debug)]
pub struct Error {
    plugin_name: String,
    cause: Cause,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            Cause::BinMissing { tried } => write!(
                f,
                "Binary not found for plugin {:?}: tried {:?}",
                self.plugin_name, tried
            ),
            Cause::ManifestFailed(err) => write!(
                f,
                "Failed to load manifest for plugin {:?}: {}",
                self.plugin_name, err
            ),
        }
    }
}

#[derive(Debug)]
pub struct Plugin {
    bin_path: PathBuf,
    manifest: Manifest,
    short_name: String,
}

impl Plugin {
    pub fn new(name: impl AsRef<str>) -> Result<Self, Error> {
        let name = name.as_ref();
        let manifest = Manifest::load_from_cargo_toml(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(&format!("../plugins/ginit-{}/Cargo.toml", name)),
        )
        .map_err(|cause| Error {
            plugin_name: name.to_owned(),
            cause: Cause::ManifestFailed(cause),
        })?;
        let bin_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(&format!("../target/debug/ginit-{}", name));
        if !bin_path.is_file() {
            return Err(Error {
                plugin_name: name.to_owned(),
                cause: Cause::BinMissing { tried: bin_path },
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

    pub fn command(
        &self,
        noise_level: opts::NoiseLevel,
        interactivity: opts::Interactivity,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> bossy::Command {
        let mut command = bossy::Command::impure(&self.bin_path);
        if let Ok(backtrace) = std::env::var("RUST_BACKTRACE") {
            command.add_env_var("RUST_BACKTRACE", backtrace);
        }
        if let Ok(log) = std::env::var("RUST_LOG") {
            command.add_env_var("RUST_LOG", log);
        }
        match noise_level {
            opts::NoiseLevel::Polite => (),
            opts::NoiseLevel::LoudAndProud => {
                command.add_arg("-v");
            }
            opts::NoiseLevel::FranklyQuitePedantic => {
                command.add_arg("-vv");
            }
        }
        match interactivity {
            opts::Interactivity::Full => (),
            opts::Interactivity::None => {
                command.add_arg("--non-interactive");
            }
        }
        command.add_args(args);
        command
    }
}
