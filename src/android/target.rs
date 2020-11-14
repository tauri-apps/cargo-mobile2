use super::{
    config::{Config, Metadata},
    env::Env,
    jnilibs::{self, JniLibs},
    ndk,
};
use crate::{
    dot_cargo::DotCargoTarget,
    opts::{ForceColor, NoiseLevel, Profile},
    target::TargetTrait,
    util::{
        cli::{Report, Reportable},
        CargoCommand,
    },
};
use once_cell_regex::exports::once_cell::sync::OnceCell;
use serde::Serialize;
use std::{collections::BTreeMap, fmt, io, str};

#[derive(Clone, Copy, Debug)]
pub enum CargoMode {
    Check,
    Build,
}

impl fmt::Display for CargoMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CargoMode::Check => write!(f, "check"),
            CargoMode::Build => write!(f, "build"),
        }
    }
}

impl CargoMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CargoMode::Check => "check",
            CargoMode::Build => "build",
        }
    }
}

#[derive(Debug)]
pub enum CompileLibError {
    MissingTool(ndk::MissingToolError),
    CargoFailed {
        mode: CargoMode,
        cause: bossy::Error,
    },
}

impl Reportable for CompileLibError {
    fn report(&self) -> Report {
        match self {
            Self::MissingTool(err) => Report::error("Failed to locate required build tool", err),
            Self::CargoFailed { mode, cause } => {
                Report::error(format!("`Failed to run `cargo {}`", mode), cause)
            }
        }
    }
}

#[derive(Debug)]
pub enum SymlinkLibsError {
    JniLibsCreationFailed(io::Error),
    SymlinkFailed(jnilibs::SymlinkLibError),
    RequiredLibsFailed(ndk::RequiredLibsError),
    LibcxxSharedPathFailed(ndk::MissingToolError),
}

impl Reportable for SymlinkLibsError {
    fn report(&self) -> Report {
        match self {
            Self::JniLibsCreationFailed(err) => {
                Report::error("Failed to create \"jniLibs\" directory", err)
            }
            Self::SymlinkFailed(err) => err.report(),
            Self::RequiredLibsFailed(err) => err.report(),
            Self::LibcxxSharedPathFailed(err) => {
                Report::error("Failed to locate \"libc++_shared.so\"", err)
            }
        }
    }
}

#[derive(Debug)]
pub enum BuildError {
    BuildFailed(CompileLibError),
    SymlinkLibsFailed(SymlinkLibsError),
}

impl Reportable for BuildError {
    fn report(&self) -> Report {
        match self {
            Self::BuildFailed(err) => err.report(),
            Self::SymlinkLibsFailed(err) => err.report(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Target<'a> {
    pub triple: &'a str,
    clang_triple_override: Option<&'a str>,
    binutils_triple_override: Option<&'a str>,
    pub abi: &'a str,
    pub arch: &'a str,
}

impl<'a> TargetTrait<'a> for Target<'a> {
    const DEFAULT_KEY: &'static str = "aarch64";

    fn all() -> &'a BTreeMap<&'a str, Self> {
        static TARGETS: OnceCell<BTreeMap<&'static str, Target<'static>>> = OnceCell::new();
        TARGETS.get_or_init(|| {
            let mut targets = BTreeMap::new();
            targets.insert(
                "aarch64",
                Target {
                    triple: "aarch64-linux-android",
                    clang_triple_override: None,
                    binutils_triple_override: None,
                    abi: "arm64-v8a",
                    arch: "arm64",
                },
            );
            targets.insert(
                "armv7",
                Target {
                    triple: "armv7-linux-androideabi",
                    clang_triple_override: Some("armv7a-linux-androideabi"),
                    binutils_triple_override: Some("arm-linux-androideabi"),
                    abi: "armeabi-v7a",
                    arch: "arm",
                },
            );
            targets.insert(
                "i686",
                Target {
                    triple: "i686-linux-android",
                    clang_triple_override: None,
                    binutils_triple_override: None,
                    abi: "x86",
                    arch: "x86",
                },
            );
            targets.insert(
                "x86_64",
                Target {
                    triple: "x86_64-linux-android",
                    clang_triple_override: None,
                    binutils_triple_override: None,
                    abi: "x86_64",
                    arch: "x86_64",
                },
            );
            targets
        })
    }

    fn triple(&'a self) -> &'a str {
        self.triple
    }

    fn arch(&'a self) -> &'a str {
        self.arch
    }
}

impl<'a> Target<'a> {
    fn clang_triple(&self) -> &'a str {
        self.clang_triple_override.unwrap_or_else(|| self.triple)
    }

    fn binutils_triple(&self) -> &'a str {
        self.binutils_triple_override.unwrap_or_else(|| self.triple)
    }

    pub fn for_abi(abi: &str) -> Option<&'a Self> {
        Self::all().values().find(|target| target.abi == abi)
    }

    pub fn generate_cargo_config(
        &self,
        config: &Config,
        env: &Env,
    ) -> Result<DotCargoTarget, ndk::MissingToolError> {
        let ar = env
            .ndk
            .binutil_path(ndk::Binutil::Ar, self.binutils_triple())?
            .display()
            .to_string();
        // Using clang as the linker seems to be the only way to get the right library search paths...
        let linker = env
            .ndk
            .compiler_path(
                ndk::Compiler::Clang,
                self.clang_triple(),
                config.min_sdk_version(),
            )?
            .display()
            .to_string();
        Ok(DotCargoTarget {
            ar: Some(ar),
            linker: Some(linker),
            rustflags: vec![
                "-Clink-arg=-landroid".to_owned(),
                "-Clink-arg=-llog".to_owned(),
                "-Clink-arg=-lOpenSLES".to_owned(),
            ],
        })
    }

    fn compile_lib(
        &self,
        config: &Config,
        metadata: &Metadata,
        env: &Env,
        noise_level: NoiseLevel,
        force_color: ForceColor,
        profile: Profile,
        mode: CargoMode,
    ) -> Result<(), CompileLibError> {
        let min_sdk_version = config.min_sdk_version();
        // Force color, since gradle would otherwise give us uncolored output
        // (which Android Studio makes red, which is extra gross!)
        let color = if force_color.yes() { "always" } else { "auto" };
        CargoCommand::new(mode.as_str())
            .with_verbose(noise_level.pedantic())
            .with_package(Some(config.app().name()))
            .with_manifest_path(Some(config.app().manifest_path()))
            .with_target(Some(self.triple))
            .with_no_default_features(metadata.no_default_features())
            .with_features(metadata.features())
            .with_release(profile.release())
            .into_command_pure(env)
            .with_env_var("ANDROID_NATIVE_API_LEVEL", min_sdk_version.to_string())
            .with_env_var(
                "TARGET_AR",
                env.ndk
                    .binutil_path(ndk::Binutil::Ar, self.binutils_triple())
                    .map_err(CompileLibError::MissingTool)?,
            )
            .with_env_var(
                "TARGET_CC",
                env.ndk
                    .compiler_path(ndk::Compiler::Clang, self.clang_triple(), min_sdk_version)
                    .map_err(CompileLibError::MissingTool)?,
            )
            .with_env_var(
                "TARGET_CXX",
                env.ndk
                    .compiler_path(ndk::Compiler::Clangxx, self.clang_triple(), min_sdk_version)
                    .map_err(CompileLibError::MissingTool)?,
            )
            .with_args(&["--color", color])
            .run_and_wait()
            .map_err(|cause| CompileLibError::CargoFailed { mode, cause })?;
        Ok(())
    }

    pub fn check(
        &self,
        config: &Config,
        metadata: &Metadata,
        env: &Env,
        noise_level: NoiseLevel,
        force_color: ForceColor,
    ) -> Result<(), CompileLibError> {
        self.compile_lib(
            config,
            metadata,
            env,
            noise_level,
            force_color,
            Profile::Debug,
            CargoMode::Check,
        )
    }

    pub fn symlink_libs(
        &self,
        config: &Config,
        ndk: &ndk::Env,
        profile: Profile,
    ) -> Result<(), SymlinkLibsError> {
        let jnilibs =
            JniLibs::create(config, *self).map_err(SymlinkLibsError::JniLibsCreationFailed)?;

        let src = config.app().prefix_path(format!(
            "target/{}/{}/{}",
            &self.triple,
            profile.as_str(),
            config.so_name(),
        ));
        jnilibs
            .symlink_lib(&src)
            .map_err(SymlinkLibsError::SymlinkFailed)?;

        let needs_cxx_shared = ndk
            .required_libs(&src, self.binutils_triple())
            .map_err(SymlinkLibsError::RequiredLibsFailed)?
            .contains("libc++_shared.so");
        if needs_cxx_shared {
            log::info!("lib {:?} requires \"libc++_shared.so\"", src);
            let cxx_shared = ndk
                .libcxx_shared_path(*self)
                .map_err(SymlinkLibsError::LibcxxSharedPathFailed)?;
            jnilibs
                .symlink_lib(&cxx_shared)
                .map_err(SymlinkLibsError::SymlinkFailed)?;
        }

        Ok(())
    }

    pub fn build(
        &self,
        config: &Config,
        metadata: &Metadata,
        env: &Env,
        noise_level: NoiseLevel,
        force_color: ForceColor,
        profile: Profile,
    ) -> Result<(), BuildError> {
        self.compile_lib(
            config,
            metadata,
            env,
            noise_level,
            force_color,
            profile,
            CargoMode::Build,
        )
        .map_err(BuildError::BuildFailed)?;
        self.symlink_libs(config, &env.ndk, profile)
            .map_err(BuildError::SymlinkLibsFailed)
    }
}
