use std::path::PathBuf;

use colored::Colorize;
use heck::ToUpperCamelCase;
use thiserror::Error;

use super::{config::Config, env::Env, target::Target};
use crate::{
    bossy,
    opts::{NoiseLevel, Profile},
    target::{get_targets, TargetInvalid, TargetTrait},
    util::{
        cli::{Report, Reportable},
        gradlew,
    },
};

#[derive(Error, Debug)]
pub enum AabError {
    #[error("Failed to bundle AAB(s): {0}")]
    BundleFailed(bossy::Error),
    #[error("Target {} is invalid; the possible targets are {}", .0.name, .0.possible.join(", "))]
    TargetInvalid(TargetInvalid),
}

impl Reportable for AabError {
    fn report(&self) -> Report {
        match self {
            Self::BundleFailed(err) => Report::error("Failed to bundle AAB(s)", err),
            Self::TargetInvalid(err) => Report::error("", err),
        }
    }
}

/// Builds AAB(s) and returns the built AAB(s) paths
pub fn build(
    config: &Config,
    env: &Env,
    noise_level: NoiseLevel,
    profile: Profile,
    targets: Vec<String>,
    split_per_abi: bool,
) -> Result<Vec<PathBuf>, AabError> {
    let profile = profile.as_str();
    let build_ty = profile.to_upper_camel_case();
    let targets = if targets.is_empty() {
        Target::all().iter().map(|t| t.1).collect()
    } else {
        get_targets::<_, _, Target, ()>(targets.iter(), None).map_err(AabError::TargetInvalid)?
    };
    println!(
        "Bundling{} AAB{} for {} ...",
        if split_per_abi { "" } else { " universal" },
        if split_per_abi { "(s)" } else { "" },
        targets
            .iter()
            .map(|t| t.triple.split("-").next().unwrap())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();
    let gradle_args = if split_per_abi {
        targets
            .iter()
            .map(|t| format!("bundle{}{}", t.arch.to_uppercase(), build_ty))
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
        .map_err(AabError::BundleFailed)?;

    let app = config.app();
    let mut outputs = Vec::new();
    if split_per_abi {
        outputs.extend(targets.iter().map(|t| {
            dunce::simplified(&app.prefix_path(format!(
                "gen/android/{}/app/build/outputs/bundle/{}{}/app-{}-{}.aab",
                app.name(),
                t.arch,
                build_ty,
                t.arch,
                profile,
            )))
            .to_path_buf()
        }));
    } else {
        outputs.push(
            dunce::simplified(&app.prefix_path(format!(
                "gen/android/{}/app/build/outputs/bundle/universal{}/app-universal-{}.aab",
                app.name(),
                build_ty,
                profile,
            )))
            .to_path_buf(),
        );
    }

    println!();
    println!("Finished bundling AAB(s):");
    for p in &outputs {
        println!("    {}", p.to_string_lossy().green(),);
    }
    Ok(outputs)
}
