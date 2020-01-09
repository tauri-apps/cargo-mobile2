use crate::{
    config::Config,
    target::{ArchiveError, BuildError, Target},
};
use ginit_core::{
    env::Env,
    exports::into_result::{command::CommandError, IntoResult as _},
    opts::Profile,
    util::pure_command::PureCommand,
};
use std::fmt;

#[derive(Debug)]
pub enum RunError {
    BuildFailed(BuildError),
    ArchiveFailed(ArchiveError),
    UnzipFailed(CommandError),
    DeployFailed(CommandError),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::BuildFailed(err) => write!(f, "Failed to build app: {}", err),
            RunError::ArchiveFailed(err) => write!(f, "Failed to archive app: {}", err),
            RunError::UnzipFailed(err) => write!(f, "Failed to unzip archive: {}", err),
            RunError::DeployFailed(err) => {
                write!(f, "Failed to deploy app via `ios-deploy`: {}", err)
            }
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
        PureCommand::new("unzip", env)
            .arg("-o") // -o = always overwrite
            .arg(&config.ipa_path())
            .arg("-d")
            .arg(&config.export_path())
            .status()
            .into_result()
            .map_err(RunError::UnzipFailed)?;
        // This dies if the device is locked, and gives you no time to react to
        // that. `ios-deploy --detect` can apparently be used to check in
        // advance, giving us an opportunity to promt. Though, it's much more
        // relaxing to just turn off auto-lock under Display & Brightness.
        PureCommand::new("ios-deploy", env)
            .args(&["--id", &self.id])
            .arg("--debug")
            .arg("--bundle")
            .arg(&config.app_path())
            // This tool can apparently install over wifi, but not debug over
            // wifi... so if your device is connected over wifi (even if it's
            // wired as well) and we're using the `--debug` flag, then
            // launching will fail unless we also specify the `--no-wifi` flag
            // to keep it from trying that.
            .arg("--no-wifi")
            .status()
            .into_result()
            .map_err(RunError::DeployFailed)
    }
}
