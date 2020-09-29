use crate::{
    apple::config::Config,
    env::{Env, ExplicitEnv as _},
    opts,
    util::cli::{Report, Reportable},
};

#[derive(Debug)]
pub enum RunAndDebugError {
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
    non_interactive: opts::NonInteractive,
    id: &str,
) -> Result<(), RunAndDebugError> {
    println!("Deploying app to device...");
    bossy::Command::pure("ios-deploy")
        .with_env_vars(env.explicit_env())
        .with_arg("--debug")
        .with_args(&["--id", id])
        .with_arg("--bundle")
        .with_arg(&config.app_path())
        .with_args(if non_interactive.yes() {
            Some("--noninteractive")
        } else {
            None
        })
        .with_arg("--no-wifi")
        .run_and_wait()
        .map(|_| ())
        .map_err(RunAndDebugError::DeployFailed)
}
