use colored::Colorize;
use heck::ToUpperCamelCase;
use thiserror::Error;

use super::{config::Config, env::Env, target::Target};
use crate::{
    opts::{NoiseLevel, Profile},
    target::{get_targets, TargetInvalid, TargetTrait},
    util::{
        cli::{Report, Reportable},
        gradlew,
    },
};

#[derive(Error, Debug)]
pub enum AabError {
    #[error("Failed to assemble AAB: {0}")]
    AssembleFailed(bossy::Error),
    #[error("Target {} is invalid; the possible targets are {}", .0.name, .0.possible.join(", "))]
    TargetInvalid(TargetInvalid),
}

impl Reportable for AabError {
    fn report(&self) -> Report {
        match self {
            Self::AssembleFailed(err) => Report::error("Failed to assemble AAB", err),
            Self::TargetInvalid(err) => Report::error("", err),
        }
    }
}

pub fn build(
    config: &Config,
    env: &Env,
    noise_level: NoiseLevel,
    profile: Profile,
    targets: Vec<String>,
    split_per_abi: bool,
) -> Result<(), AabError> {
    let profile = profile.as_str();
    let build_ty = profile.to_upper_camel_case();
    let (gradle_args, targets) = if split_per_abi {
        let targets = if targets.is_empty() {
            Target::all().iter().map(|t| t.1).collect()
        } else {
            get_targets::<_, _, Target, ()>(targets.iter(), None)
                .map_err(AabError::TargetInvalid)?
        };
        println!(
            "Building universal AABs for {} ...",
            targets
                .iter()
                .map(|t| t.triple.split("-").next().unwrap())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!();
        (
            targets
                .iter()
                .map(|t| format!("bundle{}{}", t.arch.to_uppercase(), build_ty))
                .collect(),
            targets,
        )
    } else {
        println!("Building universal AAB...");
        (vec![format!("bundleUniversal{}", build_ty)], vec![])
    };
    gradlew(config, env)
        .with_args(gradle_args)
        .with_arg(match noise_level {
            NoiseLevel::Polite => "--warn",
            NoiseLevel::LoudAndProud => "--info",
            NoiseLevel::FranklyQuitePedantic => "--debug",
        })
        .run_and_wait()
        .map_err(AabError::AssembleFailed)?;

    println!();
    println!("Finished building AABs:");
    let app = config.app();
    if split_per_abi {
        for t in targets {
            println!(
                "    {}",
                dunce::simplified(&app.prefix_path(format!(
                    "gen/android/{}/app/build/outputs/bundle/{}{}/app-{}-{}.aab",
                    app.name(),
                    t.arch,
                    build_ty,
                    t.arch,
                    profile,
                )))
                .to_string_lossy()
                .green(),
            );
        }
    } else {
        println!(
            "    {}",
            dunce::simplified(&app.prefix_path(format!(
                "gen/android/{}/app/build/outputs/bundle/universal{}/app-universal-{}.aab",
                app.name(),
                build_ty,
                profile,
            )))
            .to_string_lossy()
            .green(),
        );
    }
    Ok(())
}
