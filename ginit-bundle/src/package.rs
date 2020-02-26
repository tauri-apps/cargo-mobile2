use super::plugin::Plugin;
use ginit_core::{
    bundle::{global_config::GlobalConfig, manifest},
    exports::{
        into_result::{command::CommandError, IntoResult as _},
        toml,
    },
    opts, os,
    util::CargoCommand,
};
use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

include!(concat!(env!("OUT_DIR"), "/version.rs"));

#[derive(Debug)]
pub enum Error {
    BundleDirCreationFailed {
        package: String,
        tried: PathBuf,
        cause: io::Error,
    },
    BinBuildFailed {
        package: String,
        cause: CommandError,
    },
    BinCopyFailed {
        package: String,
        src: PathBuf,
        dest: PathBuf,
        cause: io::Error,
    },
    ManifestSerializationFailed {
        package: String,
        cause: toml::ser::Error,
    },
    ManifestWriteFailed {
        package: String,
        dest: PathBuf,
        cause: io::Error,
    },
    TemplatesCopyFailed {
        package: String,
        src: PathBuf,
        dest: PathBuf,
        cause: CommandError,
    },
    PluginsDirReadFailed {
        package: String,
        tried: PathBuf,
        cause: io::Error,
    },
    PluginsDirEntryFailed {
        package: String,
        tried: PathBuf,
        cause: io::Error,
    },
    ManifestReadFailed {
        package: String,
        tried: PathBuf,
        cause: manifest::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BundleDirCreationFailed {
                package,
                tried,
                cause,
            } => write!(
                f,
                "Failed to create bundle directory at {:?} for package {:?}: {}",
                package, tried, cause
            ),
            Self::BinBuildFailed { package, cause } => write!(
                f,
                "Failed to build binary for package {:?}: {}",
                package, cause
            ),
            Self::BinCopyFailed {
                package,
                src,
                dest,
                cause,
            } => write!(
                f,
                "Failed to copy binary for package {:?} from {:?} to {:?}: {}",
                package, src, dest, cause
            ),
            Self::ManifestSerializationFailed { package, cause } => write!(
                f,
                "Failed to serialize manifest for package {:?}: {}",
                package, cause
            ),
            Self::ManifestWriteFailed {
                package,
                dest,
                cause,
            } => write!(
                f,
                "Failed to write manifest to {:?} for package {:?}: {}",
                package, dest, cause
            ),
            Self::TemplatesCopyFailed {
                package,
                src,
                dest,
                cause,
            } => write!(
                f,
                "Failed to copy templates for package {:?} from {:?} to {:?}: {}",
                package, src, dest, cause
            ),
            Self::PluginsDirReadFailed {
                package,
                tried,
                cause,
            } => write!(
                f,
                "Failed to read plugins directory at {:?} for package {:?}: {}",
                package, tried, cause
            ),
            Self::PluginsDirEntryFailed {
                package,
                tried,
                cause,
            } => write!(
                f,
                "Failed to read plugins directory entry at {:?} for package {:?}: {}",
                package, tried, cause
            ),
            Self::ManifestReadFailed {
                package,
                tried,
                cause,
            } => write!(
                f,
                "Failed to read bundled plugin manifest at {:?} for package {:?}: {}",
                package, tried, cause
            ),
        }
    }
}

#[derive(Debug)]
pub enum Package {
    Ginit,
    BundledPlugin(Plugin),
    Plugin(Plugin),
}

impl Package {
    fn as_str(&self) -> &str {
        match self {
            Self::Ginit => ginit_core::NAME,
            Self::BundledPlugin(plugin) | Self::Plugin(plugin) => plugin.manifest.full_name(),
        }
    }

    fn bin_name(&self) -> PathBuf {
        match self {
            Self::Ginit => os::add_ext_to_bin_name(format!("cargo-{}", self.as_str())),
            Self::BundledPlugin(_) | Self::Plugin(_) => os::add_ext_to_bin_name(self.as_str()),
        }
    }

    fn bundle_name(&self, profile: opts::Profile) -> String {
        let (name, version) = match self {
            Self::Ginit | Self::BundledPlugin(_) => (ginit_core::NAME, VERSION),
            Self::Plugin(plugin) => (self.as_str(), plugin.manifest.version()),
        };
        format!("{}-{}-{}-{}", name, version, profile.as_str(), os::NAME)
    }

    fn bundle_base(&self, bundle_root: &Path, profile: opts::Profile) -> PathBuf {
        match self {
            Self::Ginit | Self::Plugin(_) => bundle_root.join(self.bundle_name(profile)),
            Self::BundledPlugin(plugin) => Self::bundle_base(&Self::Ginit, bundle_root, profile)
                .join("plugins")
                .join(plugin.manifest.full_name()),
        }
    }

    fn build_bin(&self, manifest_root: &Path, profile: opts::Profile) -> Result<(), Error> {
        CargoCommand::new("build")
            .with_package(Some(self.as_str()))
            .with_manifest_path(Some(manifest_root.join("Cargo.toml")))
            .with_release(profile.is_release())
            .into_command_impure()
            .status()
            .into_result()
            .map_err(|cause| Error::BinBuildFailed {
                package: self.as_str().to_owned(),
                cause,
            })
    }

    fn copy_bin(
        &self,
        manifest_root: &Path,
        bundle_root: &Path,
        profile: opts::Profile,
    ) -> Result<(), Error> {
        let bin = self.bin_name();
        let src = manifest_root
            .join("target")
            .join(profile.as_str())
            .join(&bin);
        let dest = self.bundle_base(bundle_root, profile).join(bin);
        fs::copy(&src, &dest).map_err(|cause| Error::BinCopyFailed {
            package: self.as_str().to_owned(),
            src,
            dest,
            cause,
        })?;
        Ok(())
    }

    fn write_manifest(&self, bundle_root: &Path, profile: opts::Profile) -> Result<(), Error> {
        if let Self::BundledPlugin(plugin) | Self::Plugin(plugin) = self {
            let dest = self
                .bundle_base(bundle_root, profile)
                .join(format!("{}.toml", plugin.manifest.full_name()));
            let ser = toml::to_string_pretty(&plugin.manifest).map_err(|cause| {
                Error::ManifestSerializationFailed {
                    package: self.as_str().to_owned(),
                    cause,
                }
            })?;
            fs::write(&dest, &ser).map_err(|cause| Error::ManifestWriteFailed {
                package: self.as_str().to_owned(),
                dest,
                cause,
            })
        } else {
            // TODO: this entire branch feels a bit silly
            let dest = self
                .bundle_base(bundle_root, profile)
                .join(format!("global-config.toml"));
            let global_config = GlobalConfig {
                default_plugins: vec![
                    "brainium".to_owned(),
                    "android".to_owned(),
                    "ios".to_owned(),
                ],
            };
            let ser = toml::to_string_pretty(&global_config).map_err(|cause| {
                Error::ManifestSerializationFailed {
                    package: self.as_str().to_owned(),
                    cause,
                }
            })?;
            fs::write(&dest, &ser).map_err(|cause| Error::ManifestWriteFailed {
                package: self.as_str().to_owned(),
                dest,
                cause,
            })
        }
    }

    fn copy_templates(
        &self,
        manifest_root: &Path,
        bundle_root: &Path,
        profile: opts::Profile,
    ) -> Result<(), Error> {
        let src = match self {
            Self::Ginit => manifest_root.join("templates"),
            Self::BundledPlugin(plugin) | Self::Plugin(plugin) => plugin.dir.join("templates"),
        };
        if src.exists() {
            let dest = self.bundle_base(bundle_root, profile);
            Command::new("cp")
                .arg("-rp")
                .args(&[&src, &dest])
                .status()
                .into_result()
                .map_err(|cause| Error::TemplatesCopyFailed {
                    package: self.as_str().to_owned(),
                    src,
                    dest,
                    cause,
                })
        } else {
            Ok(())
        }
    }

    pub fn bundle(
        &self,
        manifest_root: &Path,
        bundle_root: &Path,
        profile: opts::Profile,
    ) -> Result<(), Error> {
        let bundle_base = self.bundle_base(bundle_root, profile);
        fs::create_dir_all(&bundle_base).map_err(|cause| Error::BundleDirCreationFailed {
            package: self.as_str().to_owned(),
            tried: bundle_base,
            cause,
        })?;
        self.build_bin(manifest_root, profile)?;
        self.copy_bin(manifest_root, bundle_root, profile)?;
        self.write_manifest(bundle_root, profile)?;
        self.copy_templates(manifest_root, bundle_root, profile)?;
        if let Self::Ginit = self {
            // TODO: include ginit-bundle and ginit-install!
            let plugins_dir = manifest_root.join("plugins");
            for entry in
                fs::read_dir(&plugins_dir).map_err(|cause| Error::PluginsDirReadFailed {
                    package: self.as_str().to_owned(),
                    tried: plugins_dir.clone(),
                    cause,
                })?
            {
                let entry = entry.map_err(|cause| Error::PluginsDirEntryFailed {
                    package: self.as_str().to_owned(),
                    tried: plugins_dir.clone(),
                    cause,
                })?;
                let plugin =
                    Plugin::load(entry.path()).map_err(|cause| Error::ManifestReadFailed {
                        package: self.as_str().to_owned(),
                        tried: entry.path().clone(),
                        cause,
                    })?;
                let package = Package::BundledPlugin(plugin);
                package.bundle(manifest_root, bundle_root, profile)?;
            }
        }
        Ok(())
    }
}
