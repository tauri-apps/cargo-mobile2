use crate::{
    apple::config::Config,
    env::{Env, ExplicitEnv as _},
    opts::NoiseLevel,
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
    noise_level: NoiseLevel,
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

        let app_name = config.app().stylized_name().to_string();

        duct::cmd("idevicesyslog", ["--process", &app_name])
            .before_spawn(move |cmd| {
                if !noise_level.pedantic() {
                    // when not in pedantic log mode, filter out logs that are not from the actual app
                    // e.g. `App Name(UIKitCore)[processID]: message` vs `App Name[processID]: message`
                    cmd.arg("--match").arg(format!("{app_name}["));
                }
                Ok(())
            })
            .vars(env.explicit_env())
            .dup_stdio()
            .start()
            .map_err(RunAndDebugError::DeployFailed)
    }
}
