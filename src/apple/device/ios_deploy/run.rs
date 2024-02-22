use crate::{
    apple::config::Config,
    env::{Env, ExplicitEnv as _},
    util::cli::{Report, Reportable},
    DuctExpressionExt,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunAndDebugError {
    #[error("Failed to deploy app to device: {0}")]
    DeployFailed(std::io::Error),
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
) -> Result<duct::Handle, RunAndDebugError> {
    println!("Deploying app to device...");

    let app_path = config.app_path();
    let deploy_cmd = duct::cmd("ios-deploy", ["--debug", "--id", id, "--no-wifi"])
        .vars(env.explicit_env())
        .before_spawn(move |cmd| {
            cmd.arg("--bundle").arg(&app_path);
            if non_interactive {
                cmd.arg("--noninteractive");
            } else {
                cmd.arg("--justlaunch");
            }
            Ok(())
        })
        .dup_stdio();

    if non_interactive {
        Ok(deploy_cmd.start().map_err(RunAndDebugError::DeployFailed)?)
    } else {
        deploy_cmd
            .start()
            .map_err(RunAndDebugError::DeployFailed)?
            .wait()
            .map_err(RunAndDebugError::DeployFailed)?;
        duct::cmd("idevicesyslog", ["--process", config.app().stylized_name()])
            .vars(env.explicit_env())
            .dup_stdio()
            .start()
            .map_err(RunAndDebugError::DeployFailed)
    }
}
