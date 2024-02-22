use std::{env::temp_dir, fs::read_to_string};

use crate::{
    apple::config::Config,
    env::{Env, ExplicitEnv as _},
    util::cli::{Report, Reportable},
    DuctExpressionExt,
};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("Failed to deploy app to simulator: {0}")]
    DeployFailed(std::io::Error),
    #[error("`devicectl` returned an invalid JSON: {0}")]
    InvalidDevicectlJson(#[from] serde_json::Error),
    #[error("`devicectl` did not return the installed application metadata")]
    MissingInstalledApplication,
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::DeployFailed(err) => Report::error("Failed to deploy app to simulator", err),
            Self::InvalidDevicectlJson(err) => {
                Report::error("Failed to read `devicectl` output", err)
            }
            Self::MissingInstalledApplication => Report::error(
                "Failed to deploy application",
                "`devicectl` did not return the installed application metadata",
            ),
        }
    }
}

#[derive(Deserialize)]
struct InstalledApplication {
    #[serde(rename = "bundleID")]
    bundle_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallResult {
    installed_applications: Vec<InstalledApplication>,
}

#[derive(Deserialize)]
struct InstallOutput {
    result: InstallResult,
}

pub fn run(
    config: &Config,
    env: &Env,
    non_interactive: bool,
    id: &str,
    paired: bool,
) -> Result<duct::Handle, RunError> {
    if !paired {
        println!("Pairing with device...");

        duct::cmd("xcrun", ["devicectl", "manage", "pair", "--device", id])
            .vars(env.explicit_env())
            .dup_stdio()
            .run()
            .map_err(RunError::DeployFailed)?;
    }

    println!("Deploying app to device...");

    let app_dir = config
        .export_dir()
        .join(format!("{}_iOS.xcarchive", config.app().name()))
        .join("Products/Applications")
        .join(format!("{}.app", config.app().stylized_name()));
    let json_output_path = temp_dir().join("deviceinstall.json");
    let json_output_path_ = json_output_path.clone();
    std::fs::write(&json_output_path, "").map_err(RunError::DeployFailed)?;
    let cmd = duct::cmd(
        "xcrun",
        ["devicectl", "device", "install", "app", "--device", id],
    )
    .vars(env.explicit_env())
    .before_spawn(move |cmd| {
        cmd.arg(&app_dir)
            .arg("--json-output")
            .arg(&json_output_path_);
        Ok(())
    })
    .dup_stdio();

    cmd.run().map_err(RunError::DeployFailed)?;

    let install_output_json = read_to_string(&json_output_path).map_err(RunError::DeployFailed)?;
    let install_output = serde_json::from_str::<InstallOutput>(&install_output_json)?;
    let installed_application = install_output
        .result
        .installed_applications
        .into_iter()
        .next()
        .ok_or(RunError::MissingInstalledApplication)?;
    let app_id = installed_application.bundle_id;

    let launcher_cmd = duct::cmd(
        "xcrun",
        [
            "devicectl",
            "device",
            "process",
            "launch",
            "--device",
            id,
            &app_id,
        ],
    )
    .vars(env.explicit_env())
    .dup_stdio();

    if non_interactive {
        launcher_cmd.start().map_err(RunError::DeployFailed)
    } else {
        launcher_cmd
            .start()
            .map_err(RunError::DeployFailed)?
            .wait()
            .map_err(RunError::DeployFailed)?;

        duct::cmd("idevicesyslog", ["--process", config.app().stylized_name()])
            .vars(env.explicit_env())
            .dup_stdio()
            .start()
            .map_err(RunError::DeployFailed)
    }
}
