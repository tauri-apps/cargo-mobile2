use crate::{
    apple::config::Config,
    env::{Env, ExplicitEnv as _},
    opts::NoiseLevel,
    util::cli::{Report, Reportable},
    DuctExpressionExt,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("Failed to run command {command}: {error}")]
    CommandFailed {
        command: String,
        error: std::io::Error,
    },
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::CommandFailed { command, error } => {
                Report::error(format!("Failed to run {command}"), error)
            }
        }
    }
}

pub fn run(
    config: &Config,
    env: &Env,
    non_interactive: bool,
    noise_level: NoiseLevel,
    id: &str,
) -> Result<duct::Handle, RunError> {
    println!("Deploying app to device...");

    let app_dir = config
        .export_dir()
        .join(format!("{}_iOS.xcarchive", config.app().name()))
        .join("Products/Applications")
        .join(format!("{}.app", config.app().stylized_name()));
    let cmd = duct::cmd("xcrun", ["simctl", "install", id])
        .vars(env.explicit_env())
        .before_spawn(move |cmd| {
            cmd.arg(&app_dir);
            Ok(())
        })
        .dup_stdio();

    let handle = cmd.start().map_err(|error| RunError::CommandFailed {
        command: format!("{cmd:?}"),
        error,
    })?;

    handle.wait().map_err(|error| RunError::CommandFailed {
        command: format!("{cmd:?}"),
        error,
    })?;

    let app_id = config.app().identifier();
    let mut launcher_cmd = duct::cmd("xcrun", ["simctl", "launch", id, app_id])
        .vars(env.explicit_env())
        .dup_stdio();

    if non_interactive {
        launcher_cmd = launcher_cmd.before_spawn(|cmd| {
            cmd.arg("--console");
            Ok(())
        });
    }
    if non_interactive {
        launcher_cmd
            .start()
            .map_err(|error| RunError::CommandFailed {
                command: format!("{launcher_cmd:?}"),
                error,
            })
    } else {
        launcher_cmd
            .start()
            .map_err(|error| RunError::CommandFailed {
                command: format!("{launcher_cmd:?}"),
                error,
            })?
            .wait()
            .map_err(|error| RunError::CommandFailed {
                command: format!("{launcher_cmd:?}"),
                error,
            })?;

        let cmd = duct::cmd(
            "xcrun",
            [
                "simctl",
                "spawn",
                id,
                "log",
                "stream",
                "--level",
                "debug",
                "--predicate",
                &if noise_level.pedantic() {
                    format!("process == \"{}\"", config.app().stylized_name())
                } else {
                    format!("subsystem = \"{}\"", config.app().identifier())
                },
            ],
        )
        .vars(env.explicit_env())
        .dup_stdio();
        cmd.start().map_err(|error| RunError::CommandFailed {
            command: format!("{cmd:?}"),
            error,
        })
    }
}
