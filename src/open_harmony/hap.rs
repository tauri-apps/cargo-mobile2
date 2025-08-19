use std::path::PathBuf;

use colored::Colorize;
use heck::ToUpperCamelCase;
use thiserror::Error;

use super::{config::Config, env::Env, target::Target};
use crate::{
    opts::{NoiseLevel, Profile},
    util::{
        cli::{Report, Reportable},
        hvigorw, last_modified, prefix_path,
    },
};

#[derive(Debug, Error)]
pub enum HapError {
    #[error("Failed to assemble HAP: {0}")]
    AssembleFailed(#[from] std::io::Error),
}

impl Reportable for HapError {
    fn report(&self) -> Report {
        match self {
            Self::AssembleFailed(err) => Report::error("Failed to assemble HAP", err),
        }
    }
}

pub fn haps_paths(config: &Config, profile: Profile, flavor: &str) -> Vec<PathBuf> {
    profile
        .suffixes()
        .iter()
        .map(|suffix| {
            prefix_path(
                config.project_dir(),
                format!(
                    "app/build/outputs/apk/{}/{}/app-{}-{}.{}",
                    flavor,
                    profile.as_str(),
                    flavor,
                    suffix,
                    "apk"
                ),
            )
        })
        .collect()
}

/// Builds HAP(s) and returns the built HAP(s) paths
pub fn build(
    config: &Config,
    env: &Env,
    noise_level: NoiseLevel,
    profile: Profile,
    targets: Vec<&Target>,
    split_per_abi: bool,
) -> Result<Vec<PathBuf>, HapError> {
    let build_ty = profile.as_str().to_upper_camel_case();

    let hvigor_args = if split_per_abi {
        targets
            .iter()
            .map(|t| format!("assemble{}{}", t.arch_upper_camel_case(), build_ty))
            .collect()
    } else {
        let mut args = vec![format!("assembleUniversal{}", build_ty)];

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

    hvigorw(config, env)
    .before_spawn(move |cmd| {
        cmd.args(&hvigor_args).arg(match noise_level {
            NoiseLevel::Polite => "--warn",
            NoiseLevel::LoudAndProud => "--info",
            NoiseLevel::FranklyQuitePedantic => "--debug",
        });
        Ok(())
    })
    .start()
    .inspect_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
           log::error!("`hvigorw` not found. Make sure you have the OpenHarmony command line tools installed and added to your PATH");
        }
    })?
    .wait()?;

    let mut outputs = Vec::new();
    if split_per_abi {
        let paths = targets
            .iter()
            .map(|t| {
                haps_paths(config, profile, t.arch)
                    .into_iter()
                    .reduce(last_modified)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        outputs.extend(paths);
    } else {
        let path = haps_paths(config, profile, "universal")
            .into_iter()
            .reduce(last_modified)
            .unwrap();
        outputs.push(path);
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
    ) -> Result<(), HapError> {
        println!(
            "Building{} HAP{} for {} ...\n",
            if split_per_abi { "" } else { " universal" },
            if split_per_abi { "(s)" } else { "" },
            targets
                .iter()
                .map(|t| t.triple.split('-').next().unwrap())
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
