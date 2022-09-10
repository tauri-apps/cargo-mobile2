use crate::{
    apple::config::Config,
    bossy,
    env::{Env, ExplicitEnv as _},
    util::{Report, Reportable},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunAndDebugError {
    #[error("Failed to deploy app to device: {0}")]
    DeployFailed(bossy::Error),
}

impl Reportable for RunAndDebugError {
    fn report(&self) -> Report {
        match self {
            Self::DeployFailed(err) => Report::error("Failed to deploy app to device", err),
        }
    }
}

pub fn run_and_debug(
    config: &Config,
    env: &Env,
    non_interactive: bool,
    id: &str,
) -> Result<bossy::Handle, RunAndDebugError> {
    println!("Deploying app to device...");
    bossy::Command::pure("ios-deploy")
        .with_env_vars(env.explicit_env())
        .with_arg("--debug")
        .with_args(&["--id", id])
        .with_arg("--bundle")
        .with_arg(&config.app_path())
        .with_args(if non_interactive {
            Some("--noninteractive")
        } else {
            None
        })
        .with_arg("--no-wifi")
        .run()
        .map_err(RunAndDebugError::DeployFailed)
}
