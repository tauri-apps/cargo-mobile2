use super::{
    config::Config,
    deps::{GemCache, PackageSpec},
    target::{ArchiveError, BuildError, ExportError, Target},
};
use crate::{
    apple::target::ExportConfig,
    env::{Env, ExplicitEnv as _},
    opts,
    util::cli::{Report, Reportable},
    DuctExpressionExt,
};
use std::{
    collections::BTreeSet,
    fmt::{self, Display},
    path::PathBuf,
};
use thiserror::Error;

mod devicectl;
mod ios_deploy;
mod simctl;

pub use simctl::Device as Simulator;

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
    #[error("{0}")]
    DeployFailed(String),
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
            Self::DeployFailed(err) => Report::error("Failed to deploy app", err),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum DeviceKind {
    Simulator,
    IosDeployDevice,
    DeviceCtlDevice,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Device<'a> {
    id: String,
    name: String,
    model: String,
    target: &'a Target<'a>,
    kind: DeviceKind,
    paired: bool,
}

impl<'a> Display for Device<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.model)
    }
}

impl<'a> Device<'a> {
    pub(super) fn new(
        id: String,
        name: String,
        model: String,
        target: &'a Target<'a>,
        kind: DeviceKind,
    ) -> Self {
        Self {
            id,
            name,
            model,
            target,
            kind,
            paired: true,
        }
    }

    pub fn paired(mut self, paired: bool) -> Self {
        self.paired = paired;
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

    pub fn kind(&self) -> DeviceKind {
        self.kind
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

        match self.kind {
            DeviceKind::Simulator => simctl::run(config, env, non_interactive, &self.id)
                .map_err(|e| RunError::DeployFailed(e.to_string())),
            DeviceKind::IosDeployDevice | DeviceKind::DeviceCtlDevice => {
                println!("Exporting app...");
                self.target
                    .export(config, env, noise_level, ExportConfig::default())
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
                    })
                    .dup_stdio();

                cmd.run().map_err(RunError::UnzipFailed)?;

                if self.kind == DeviceKind::IosDeployDevice {
                    ios_deploy::run_and_debug(config, env, non_interactive, &self.id)
                        .map_err(|e| RunError::DeployFailed(e.to_string()))
                } else {
                    devicectl::run(config, env, non_interactive, &self.id, self.paired)
                        .map_err(|e| RunError::DeployFailed(e.to_string()))
                }
            }
        }
    }
}

pub fn list_devices<'a>(env: &Env) -> Result<BTreeSet<Device<'a>>, String> {
    let mut devices = BTreeSet::default();
    let mut error = None;

    // devicectl
    match devicectl::device_list(env) {
        Ok(d) => {
            devices.extend(d);
        }
        Err(e) => {
            error = Some(e);
        }
    }

    // if we could not find a device with devicectl, let's use ios-deploy
    if devices.is_empty() {
        PackageSpec::brew("ios-deploy")
            .install(false, &mut GemCache::new())
            .map_err(|e| e.to_string())?;
        return ios_deploy::device_list(env).map_err(|e| e.to_string());
    }

    if let Some(err) = error {
        Err(err.to_string())
    } else {
        Ok(devices)
    }
}

pub fn list_simulators(env: &Env) -> Result<BTreeSet<Simulator>, String> {
    simctl::device_list(env).map_err(|e| e.to_string())
}
