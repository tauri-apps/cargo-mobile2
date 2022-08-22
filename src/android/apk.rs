use std::path::PathBuf;

use colored::Colorize;
use heck::ToUpperCamelCase;
use thiserror::Error;

use super::{config::Config, env::Env, jnilibs, target::Target};
use crate::{
    android::jnilibs::JniLibs,
    bossy,
    opts::{NoiseLevel, Profile},
    target::TargetTrait,
    util::{
        cli::{Report, Reportable},
        gradlew, prefix_path,
    },
};

#[derive(Debug, Error)]
pub enum ApkBuildError {
    #[error(transparent)]
    LibSymlinkCleaningFailed(jnilibs::RemoveBrokenLinksError),
    #[error("Failed to assemble APK: {0}")]
    AssembleFailed(bossy::Error),
}

impl Reportable for ApkBuildError {
    fn report(&self) -> Report {
        match self {
            Self::LibSymlinkCleaningFailed(err) => err.report(),
            Self::AssembleFailed(err) => Report::error("Failed to assemble APK", err),
        }
    }
}

#[derive(Debug, Error)]

pub enum ApkError {
    #[error(transparent)]
    ApkBuildError(ApkBuildError),
}

impl Reportable for ApkError {
    fn report(&self) -> Report {
        match self {
            Self::ApkBuildError(err) => err.report(),
        }
    }
}

pub fn apk_path(config: &Config, profile: Profile, flavor: &str) -> PathBuf {
    prefix_path(
        config.project_dir(),
        format!(
            "app/build/outputs/{}/app-{}-{}.{}",
            format!("apk/{}/{}", flavor, profile.as_str()),
            flavor,
            profile.suffix(),
            "apk"
        ),
    )
}

/// Builds APK(s) and returns the built APK(s) paths
pub fn build<'a>(
    config: &Config,
    env: &Env,
    noise_level: NoiseLevel,
    profile: Profile,
    targets: Vec<&Target>,
    split_per_abi: bool,
) -> Result<Vec<PathBuf>, ApkError> {
    JniLibs::remove_broken_links(config)
        .map_err(ApkBuildError::LibSymlinkCleaningFailed)
        .map_err(ApkError::ApkBuildError)?;

    let build_ty = profile.as_str().to_upper_camel_case();

    let gradle_args = if split_per_abi {
        targets
            .iter()
            .map(|t| format!("assemble{}{}", t.arch_upper_camel_case(), build_ty))
            .collect()
    } else {
        vec![
            format!("assembleUniversal{}", build_ty),
            format!(
                "-PabiList={}",
                targets.iter().map(|t| t.abi).collect::<Vec<_>>().join(",")
            ),
        ]
    };
    gradlew(config, env)
        .with_args(gradle_args)
        .with_arg(match noise_level {
            NoiseLevel::Polite => "--warn",
            NoiseLevel::LoudAndProud => "--info",
            NoiseLevel::FranklyQuitePedantic => "--debug",
        })
        .run_and_wait()
        .map_err(ApkBuildError::AssembleFailed)
        .map_err(ApkError::ApkBuildError)?;

    let mut outputs = Vec::new();
    if split_per_abi {
        outputs.extend(
            targets
                .iter()
                .map(|t| dunce::simplified(&apk_path(config, profile, t.arch)).to_path_buf()),
        );
    } else {
        outputs.push(dunce::simplified(&apk_path(config, profile, "universal")).to_path_buf());
    }

    Ok(outputs)
}

pub mod cli {
    use super::*;
    pub fn build(
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
        targets: Vec<&Target>,
        split_per_abi: bool,
    ) -> Result<(), ApkError> {
        println!(
            "Building{} APK{} for {} ...\n",
            if split_per_abi { "" } else { " universal" },
            if split_per_abi { "(s)" } else { "" },
            targets
                .iter()
                .map(|t| t.triple.split("-").next().unwrap())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let outputs = super::build(config, env, noise_level, profile, targets, split_per_abi)?;

        println!("\nFinished building APK(s):");
        for p in &outputs {
            println!("    {}", p.to_string_lossy().green(),);
        }
        Ok(())
    }
}
