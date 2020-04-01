use super::{
    config::Config,
    target::{ArchiveError, BuildError, Target},
};
use crate::{
    env::{Env, ExplicitEnv as _},
    opts::Profile,
};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum RunError {
    BuildFailed(BuildError),
    ArchiveFailed(ArchiveError),
    UnzipFailed(bossy::Error),
    DeployFailed(bossy::Error),
}

impl Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuildFailed(err) => write!(f, "Failed to build app: {}", err),
            Self::ArchiveFailed(err) => write!(f, "Failed to archive app: {}", err),
            Self::UnzipFailed(err) => write!(f, "Failed to unzip archive: {}", err),
            Self::DeployFailed(err) => write!(f, "Failed to deploy app via `ios-deploy`: {}", err),
        }
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Device<'a> {
    id: String,
    name: String,
    model: String,
    target: &'a Target<'a>,
}

impl<'a> fmt::Display for Device<'a> {
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
        }
    }

    pub fn target(&self) -> &'a Target<'a> {
        self.target
    }

    pub fn run(&self, config: &Config, env: &Env, profile: Profile) -> Result<(), RunError> {
        // TODO: These steps are run unconditionally, which is slooooooow
        self.target
            .build(config, env, profile)
            .map_err(RunError::BuildFailed)?;
        self.target
            .archive(config, env, profile)
            .map_err(RunError::ArchiveFailed)?;
        bossy::Command::pure("unzip")
            .with_env_vars(env.explicit_env())
            .with_arg("-o") // -o = always overwrite
            .with_arg(&config.ipa_path())
            .with_arg("-d")
            .with_arg(&config.export_dir())
            .run_and_wait()
            .map_err(RunError::UnzipFailed)?;
        // This dies if the device is locked, and gives you no time to react to
        // that. `ios-deploy --detect` can apparently be used to check in
        // advance, giving us an opportunity to promt. Though, it's much more
        // relaxing to just turn off auto-lock under Display & Brightness.
        bossy::Command::pure("ios-deploy")
            .with_env_vars(env.explicit_env())
            .with_args(&["--id", &self.id])
            .with_arg("--debug")
            .with_arg("--bundle")
            .with_arg(&config.app_path())
            // This tool can apparently install over wifi, but not debug over
            // wifi... so if your device is connected over wifi (even if it's
            // wired as well) and we're using the `--debug` flag, then
            // launching will fail unless we also specify the `--no-wifi` flag
            // to keep it from trying that.
            .with_arg("--no-wifi")
            .run_and_wait()
            .map_err(RunError::DeployFailed)?;
        Ok(())
    }
}
