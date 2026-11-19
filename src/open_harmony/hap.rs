use std::path::PathBuf;

use colored::Colorize;
use thiserror::Error;

use super::{config::Config, env::Env};
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

pub fn haps_paths(config: &Config) -> Vec<PathBuf> {
    let output_dir = prefix_path(config.project_dir(), "entry/build/default/outputs/default");
    vec![
        output_dir.join("entry-default-signed.hap"),
        output_dir.join("entry-default-unsigned.hap"),
    ]
}

/// Builds HAP(s) and returns the built HAP(s) paths
pub fn build(
    config: &Config,
    env: &Env,
    noise_level: NoiseLevel,
    profile: Profile,
) -> Result<Vec<PathBuf>, HapError> {
    super::ohpm::install(config, env)?;

    let build_mode = profile.as_str().to_lowercase();

    let hvigor_args = vec![
        "--mode".to_string(),
        "module".to_string(),
        "assembleHap".to_string(),
        "--parallel".to_string(),
        "--incremental".to_string(),
        "-p".to_string(),
        format!("buildMode={build_mode}"),
    ];

    hvigorw(config, env)
    .before_spawn(move |cmd| {
        cmd.args(&hvigor_args).arg(match noise_level {
            NoiseLevel::Polite => "--info",
            NoiseLevel::LoudAndProud | NoiseLevel::FranklyQuitePedantic => "--debug",
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

    let path = haps_paths(config)
        .into_iter()
        .reduce(last_modified)
        .unwrap();
    outputs.push(path);

    Ok(outputs)
}

pub mod cli {
    use super::*;
    pub fn build(
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<(), HapError> {
        println!("Building HAP...\n");

        let outputs = super::build(config, env, noise_level, profile)?;

        println!("\nFinished building APK(s):");
        for p in &outputs {
            println!("    {}", p.to_string_lossy().green(),);
        }
        Ok(())
    }
}
