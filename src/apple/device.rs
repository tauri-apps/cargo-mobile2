use super::{
    config::Config,
    ios_deploy, simctl,
    target::{ArchiveError, BuildError, ExportError, Target},
};
use crate::{
    env::{Env, ExplicitEnv as _},
    opts,
    util::cli::{Report, Reportable},
    DuctExpressionExt,
};
use std::{
    fmt::{self, Display},
    path::PathBuf,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunError {
    #[error(transparent)]
    BuildFailed(BuildError),
    #[error(transparent)]
    ArchiveFailed(ArchiveError),
    #[error(transparent)]
    ExportFailed(ExportError),
    #[error("IPA appears to be missing. Not found at either {old} or {new}")]
    IpaMissing { old: PathBuf, new: PathBuf },
    #[error("Failed to unzip archive: {0}")]
    UnzipFailed(std::io::Error),
    #[error(transparent)]
    DeployFailed(ios_deploy::RunAndDebugError),
    #[error(transparent)]
    SimulatorDeployFailed(simctl::RunError),
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::BuildFailed(err) => err.report(),
            Self::ArchiveFailed(err) => err.report(),
            Self::ExportFailed(err) => err.report(),
            Self::IpaMissing { old, new } => Report::error(
                "IPA appears to be missing",
                format!("Not found at either {:?} or {:?}", old, new),
            ),
            Self::UnzipFailed(err) => Report::error("Failed to unzip archive", err),
            Self::DeployFailed(err) => err.report(),
            Self::SimulatorDeployFailed(err) => err.report(),
        }
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Device<'a> {
    id: String,
    name: String,
    model: String,
    target: &'a Target<'a>,
    simulator: bool,
}

impl<'a> Display for Device<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.model)
    }
}

impl<'a> Device<'a> {
    pub(super) fn new(id: String, name: String, model: String, target: &'a Target<'a>) -> Self {
        Self {
            id,
            name,
            model,
            target,
            simulator: false,
        }
    }

    pub fn simulator(mut self) -> Self {
        self.simulator = true;
        self
    }

    pub fn target(&self) -> &'a Target<'a> {
        self.target
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn run(
        &self,
        config: &Config,
        env: &Env,
        noise_level: opts::NoiseLevel,
        non_interactive: bool,
        profile: opts::Profile,
    ) -> Result<duct::Handle, RunError> {
        // TODO: These steps are run unconditionally, which is slooooooow
        println!("Building app...");
        self.target
            .build(config, env, noise_level, profile)
            .map_err(RunError::BuildFailed)?;
        println!("Archiving app...");
        self.target
            .archive(config, env, noise_level, profile, None)
            .map_err(RunError::ArchiveFailed)?;

        if self.simulator {
            simctl::run(config, env, non_interactive, &self.id)
                .map_err(RunError::SimulatorDeployFailed)
        } else {
            println!("Exporting app...");
            self.target
                .export(config, env, noise_level)
                .map_err(RunError::ExportFailed)?;
            println!("Extracting IPA...");

            let ipa_path = config
                .ipa_path()
                .map_err(|(old, new)| RunError::IpaMissing { old, new })?;
            let export_dir = config.export_dir();
            let cmd = duct::cmd::<&str, [String; 0]>("unzip", [])
                .vars(env.explicit_env())
                .before_spawn(move |cmd| {
                    if noise_level.pedantic() {
                        cmd.arg("-q");
                    }
                    cmd.arg("-o").arg(&ipa_path).arg("-d").arg(&export_dir);
                    Ok(())
                });

            cmd.start()
                .map_err(RunError::UnzipFailed)?
                .wait()
                .map_err(RunError::UnzipFailed)?;

            ios_deploy::run_and_debug(config, env, non_interactive, &self.id)
                .map_err(RunError::DeployFailed)
        }
    }
}
