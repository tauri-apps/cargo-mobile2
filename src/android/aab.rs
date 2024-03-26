use std::path::PathBuf;

use colored::Colorize;
use heck::ToUpperCamelCase;
use thiserror::Error;

use super::{config::Config, env::Env, target::Target};
use crate::{
    opts::{NoiseLevel, Profile},
    util::{
        cli::{Report, Reportable},
        gradlew, prefix_path,
    },
};

#[derive(Debug, Error)]
pub enum AabError {
    #[error("Failed to build AAB: {0}")]
    BuildFailed(#[from] std::io::Error),
}

impl Reportable for AabError {
    fn report(&self) -> Report {
        match self {
            Self::BuildFailed(err) => Report::error("Failed to build AAB", err),
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
            .map(|t| format!("bundle{}{}", t.arch_upper_camel_case(), build_ty))
            .collect()
    } else {
        let mut args = vec![format!("bundleUniversal{}", build_ty)];

        if !targets.is_empty() {
            args.extend_from_slice(&[
                format!(
                    "-PabiList={}",
                    targets.iter().map(|t| t.abi).collect::<Vec<_>>().join(",")
                ),
                format!(
                    "-ParchList={}",
                    targets.iter().map(|t| t.arch).collect::<Vec<_>>().join(",")
                ),
                format!(
                    "-PtargetList={}",
                    targets
                        .iter()
                        .map(|t| t.triple.split('-').next().unwrap())
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            ])
        }

        args
    };
    gradlew(config, env)
        .before_spawn(move |cmd| {
            cmd.args(&gradle_args).arg(match noise_level {
                NoiseLevel::Polite => "--warn",
                NoiseLevel::LoudAndProud => "--info",
                NoiseLevel::FranklyQuitePedantic => "--debug",
            });
            Ok(())
        })
        .start()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
               log::error!("`gradlew` not found. Make sure you have the Android SDK installed and added to your PATH");
            }
            err
        })?
        .wait()?;

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
            "app/build/outputs/bundle/{}{}/app-{}-{}.{}",
            flavor,
            profile.as_str_pascal_case(),
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
                .map(|t| t.triple.split('-').next().unwrap())
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
