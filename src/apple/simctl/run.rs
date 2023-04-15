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

pub fn run(
    config: &Config,
    env: &Env,
    non_interactive: bool,
    id: &str,
) -> Result<bossy::Handle, RunError> {
    println!("Deploying app to device...");
    let handle = bossy::Command::pure("xcrun")
        .with_env_vars(env.explicit_env())
        .with_args(["simctl", "install", id])
        .with_arg(
            config
                .export_dir()
                .join(format!("{}_iOS.xcarchive", config.app().name()))
                .join("Products/Applications")
                .join(format!("{}.app", config.app().name())),
        )
        .run()
        .map_err(RunError::DeployFailed)?;

    handle.wait().map_err(RunError::DeployFailed)?;

    let app_id = format!("{}.{}", config.app().reverse_domain(), config.app().name());

    let mut launcher_cmd = bossy::Command::pure("xcrun")
        .with_env_vars(env.explicit_env())
        .with_args(["simctl", "launch"])
        .with_args(if non_interactive {
            Some("--console")
        } else {
            None
        })
        .with_arg(id)
        .with_arg(&app_id);
    if non_interactive {
        launcher_cmd.run().map_err(RunError::DeployFailed)
    } else {
        launcher_cmd
            .run_and_wait()
            .map_err(RunError::DeployFailed)?;
        bossy::Command::pure("xcrun")
            .with_env_vars(env.explicit_env())
            .with_args(["simctl", "spawn", id, "log", "stream"])
            .with_args(["--level", "debug"])
            .with_arg("--predicate")
            .with_arg(format!("subsystem == \"{app_id}\""))
            .run()
            .map_err(RunError::DeployFailed)
    }
}
