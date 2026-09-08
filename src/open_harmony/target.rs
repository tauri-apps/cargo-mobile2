use super::{
    config::{Config, Metadata},
    env::Env,
};
use crate::{
    env::ExplicitEnv,
    opts::{NoiseLevel, Profile},
    target::TargetTrait,
    util::cli::{Report, Reportable},
    DuctExpressionExt,
};
use once_cell::sync::OnceCell;
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
    #[error("`Failed to run `ohrs {mode}`: {cause}")]
    OhrsFailed {
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
                    triple: "aarch64-unknown-linux-ohos",
                    abi: "arm64-v8a",
                    arch: "arm64",
                },
            );
            targets.insert(
                "armv7",
                Target {
                    triple: "aarch64-unknown-linux-ohos",
                    abi: "armeabi-v7a",
                    arch: "arm",
                },
            );
            targets.insert(
                "x86_64",
                Target {
                    triple: "x86_64-unknown-linux-ohos",
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
        // Force color, since gradle would otherwise give us uncolored output
        // (which Android Studio makes red, which is extra gross!)
        let color = if force_color { "always" } else { "auto" };

        let mut cargo_args: Vec<String> = vec![
            "--package".into(),
            config.app().name().into(),
            "--manifest-path".into(),
            config.app().manifest_path().to_str().unwrap().into(),
            "--color".into(),
            color.into(),
        ];
        if noise_level.pedantic() {
            cargo_args.push("-vv".into());
        }
        if let Some(args) = metadata.cargo_args() {
            cargo_args.extend_from_slice(args);
        }
        if let Some(features) = metadata.features() {
            let features = features.join(" ");
            cargo_args.extend_from_slice(&["--features".into(), features.as_str().to_string()]);
        }
        if profile.release() {
            cargo_args.push("--release".into());
        }
        if metadata.no_default_features() {
            cargo_args.push("--no-default-features".into());
        }

        let dist = config.project_dir().join("entry").join("libs");

        duct::cmd("ohrs", ["build", "--arch", self.arch])
            .before_spawn(move |cmd| {
                cmd.arg("--dist").arg(&dist).arg("--").args(&cargo_args);
                Ok(())
            })
            .vars(env.explicit_env())
            .run()
            .map_err(|cause| CompileLibError::OhrsFailed { mode, cause })?;

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
        .map_err(BuildError::BuildFailed)
    }
}
