use crate::{
    apple::config::Config,
    bossy,
    env::{Env, ExplicitEnv as _},
    util::cli::{Report, Reportable},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("Failed to deploy app to simulator: {0}")]
    DeployFailed(bossy::Error),
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::DeployFailed(err) => Report::error("Failed to deploy app to simulator", err),
        }
    }
}

pub fn run(config: &Config, env: &Env, id: &str) -> Result<bossy::Handle, RunError> {
    println!("Deploying app to device...");
    let handle = bossy::Command::pure("xcrun")
        .with_env_vars(env.explicit_env())
        .with_args(&["simctl", "install", id])
        .with_arg(&config.app_path())
        .run()
        .map_err(RunError::DeployFailed)?;

    handle.wait().map_err(RunError::DeployFailed)?;

    bossy::Command::pure("xcrun")
        .with_env_vars(env.explicit_env())
        .with_args(&["simctl", "launch", id])
        .with_arg(format!(
            "{}.{}",
            config.app().reverse_domain(),
            config.app().name()
        ))
        .run()
        .map_err(RunError::DeployFailed)
}
