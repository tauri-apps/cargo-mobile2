use super::{
    config::{Config, Metadata},
    env::Env,
    jnilibs::{self, JniLibs},
    ndk,
};
use crate::{
    dot_cargo::DotCargoTarget,
    opts::{NoiseLevel, Profile},
    target::TargetTrait,
    util::{
        cli::{Report, Reportable},
        CargoCommand,
    },
};
use once_cell_regex::exports::once_cell::sync::OnceCell;
use serde::Serialize;
use std::{collections::BTreeMap, fmt, io, path::PathBuf, str};
use thiserror::Error;

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

#[derive(Debug, Error)]
pub enum CompileLibError {
    #[error("Failed to locate required build tool: {0}")]
    MissingTool(ndk::MissingToolError),
    #[error("`Failed to run `cargo {mode}`: {cause}")]
    CargoFailed {
        mode: CargoMode,
        cause: std::io::Error,
    },
    #[error("`Failed to write file at {path} : {cause}")]
    FileWrite { path: PathBuf, cause: io::Error },
}

impl Reportable for CompileLibError {
    fn report(&self) -> Report {
        Report::error("Failed to compile lib", self)
    }
}

#[derive(Debug, Error)]
pub enum SymlinkLibsError {
    #[error("Failed to create \"jniLibs\" directory: {0}")]
    JniLibsCreationFailed(io::Error),
    #[error(transparent)]
    SymlinkFailed(jnilibs::SymlinkLibError),
    #[error(transparent)]
    RequiredLibsFailed(ndk::RequiredLibsError),
    #[error("Failed to locate \"libc++_shared.so\": {0}")]
    LibcxxSharedPathFailed(ndk::MissingToolError),
    #[error("Library artifact not found at {path}. Make sure your Cargo.toml file has a [lib] block with `crate-type = [\"staticlib\", \"cdylib\", \"rlib\"]`")]
    LibNotFound { path: PathBuf },
}

impl Reportable for SymlinkLibsError {
    fn report(&self) -> Report {
        Report::error("Failed to symlink lib", self)
    }
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error(transparent)]
    BuildFailed(CompileLibError),
    #[error(transparent)]
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

    fn name_list() -> Vec<&'a str> {
        Self::all().keys().copied().collect::<Vec<_>>()
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
        self.clang_triple_override.unwrap_or(self.triple)
    }

    fn binutils_triple(&self) -> &'a str {
        self.binutils_triple_override.unwrap_or(self.triple)
    }

    pub fn for_abi(abi: &str) -> Option<&'a Self> {
        Self::all().values().find(|target| target.abi == abi)
    }

    pub fn arch_upper_camel_case(&'a self) -> &'a str {
        match self.arch() {
            "arm" => "Arm",
            "arm64" => "Arm64",
            "x86_64" => "X86_64",
            "x86" => "X86",
            arch => arch,
        }
    }

    pub fn generate_cargo_config(
        &self,
        config: &Config,
        env: &Env,
    ) -> Result<DotCargoTarget, ndk::MissingToolError> {
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
            linker: Some(linker),
            rustflags: vec![
                "-L".to_owned(),
                "-Clink-arg=-landroid".to_owned(),
                "-Clink-arg=-llog".to_owned(),
                "-Clink-arg=-lOpenSLES".to_owned(),
            ],
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn compile_lib(
        &self,
        config: &Config,
        metadata: &Metadata,
        env: &Env,
        noise_level: NoiseLevel,
        force_color: bool,
        profile: Profile,
        mode: CargoMode,
    ) -> Result<(), CompileLibError> {
        let min_sdk_version = config.min_sdk_version();

        // Force color, since gradle would otherwise give us uncolored output
        // (which Android Studio makes red, which is extra gross!)
        let color = if force_color { "always" } else { "auto" };
        CargoCommand::new(mode.as_str())
            .with_verbose(noise_level.pedantic())
            .with_package(Some(config.app().name()))
            .with_manifest_path(Some(config.app().manifest_path()))
            .with_target(Some(self.triple))
            .with_no_default_features(metadata.no_default_features())
            .with_args(metadata.cargo_args())
            .with_features(metadata.features())
            .with_release(profile.release())
            .build(env)
            .env("ANDROID_NATIVE_API_LEVEL", min_sdk_version.to_string())
            .env(
                "TARGET_AR",
                env.ndk
                    .ar_path(self.triple)
                    .map_err(CompileLibError::MissingTool)?,
            )
            .env(
                "TARGET_CC",
                env.ndk
                    .compiler_path(ndk::Compiler::Clang, self.clang_triple(), min_sdk_version)
                    .map_err(CompileLibError::MissingTool)?,
            )
            .env(
                "TARGET_CXX",
                env.ndk
                    .compiler_path(ndk::Compiler::Clangxx, self.clang_triple(), min_sdk_version)
                    .map_err(CompileLibError::MissingTool)?,
            )
            .before_spawn(move |cmd| {
                cmd.args(["--color", color]);
                Ok(())
            })
            .run()
            .map_err(|cause| CompileLibError::CargoFailed { mode, cause })?;
        Ok(())
    }

    pub fn check(
        &self,
        config: &Config,
        metadata: &Metadata,
        env: &Env,
        noise_level: NoiseLevel,
        force_color: bool,
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

        let src = config
            .app()
            .target_dir(self.triple, profile)
            .join(config.so_name());

        if !src.exists() {
            return Err(SymlinkLibsError::LibNotFound { path: src });
        }

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
        force_color: bool,
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
