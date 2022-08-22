use std::path::PathBuf;

use colored::Colorize;
use heck::ToUpperCamelCase;
use thiserror::Error;

use super::{config::Config, env::Env, target::Target};
use crate::{
    bossy,
    opts::{NoiseLevel, Profile},
    target::TargetTrait,
    util::{
        cli::{Report, Reportable},
        gradlew, prefix_path,
    },
};

#[derive(Debug, Error)]
pub enum AabBuildError {
    #[error("Failed to build AAB: {0}")]
    BuildFailed(bossy::Error),
}

impl Reportable for AabBuildError {
    fn report(&self) -> Report {
        match self {
            Self::BuildFailed(err) => Report::error("Failed to build AAB", err),
        }
    }
}

#[derive(Debug, Error)]
pub enum AabError {
    #[error(transparent)]
    AabBuildError(AabBuildError),
}

impl Reportable for AabError {
    fn report(&self) -> Report {
        match self {
            Self::AabBuildError(err) => err.report(),
        }
    }
}

/// Builds AAB(s) and returns the built AAB(s) paths
pub fn build(
    config: &Config,
    env: &Env,
    noise_level: NoiseLevel,
    profile: Profile,
    targets: Vec<&Target>,
    split_per_abi: bool,
) -> Result<Vec<PathBuf>, AabError> {
    let build_ty = profile.as_str().to_upper_camel_case();

    let gradle_args = if split_per_abi {
        targets
            .iter()
            .map(|t| format!("bundle{}{}", t.arch_camel_case(), build_ty))
            .collect()
    } else {
        vec![
            format!("bundleUniversal{}", build_ty),
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
        .map_err(AabBuildError::BuildFailed)
        .map_err(AabError::AabBuildError)?;

    let mut outputs = Vec::new();
    if split_per_abi {
        outputs.extend(
            targets
                .iter()
                .map(|t| dunce::simplified(&aab_path(config, profile, t.arch)).to_path_buf()),
        );
    } else {
        outputs.push(dunce::simplified(&aab_path(config, profile, "universal")).to_path_buf());
    }

    Ok(outputs)
}

pub fn aab_path(config: &Config, profile: Profile, flavor: &str) -> PathBuf {
    prefix_path(
        config.project_dir(),
        format!(
            "app/build/outputs/{}/app-{}-{}.{}",
            format!("bundle/{}{}", flavor, profile.as_str_pascal_case()),
            flavor,
            profile.as_str(),
            "aab"
        ),
    )
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
    ) -> Result<(), AabError> {
        println!(
            "Building{} AAB{} for {} ...\n",
            if split_per_abi { "" } else { " universal" },
            if split_per_abi { "(s)" } else { "" },
            targets
                .iter()
                .map(|t| t.triple.split("-").next().unwrap())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let outputs = super::build(config, env, noise_level, profile, targets, split_per_abi)?;

        println!("\nFinished building AAB(s):");
        for p in &outputs {
            println!("    {}", p.to_string_lossy().green(),);
        }
        Ok(())
    }
}
