use super::{error, Event};
use crate::{
    apple::config::Config,
    env::{Env, ExplicitEnv as _},
    opts,
    util::{
        cli::{Report, Reportable, TextWrapper},
        prompt,
    },
};

#[derive(Debug)]
pub enum RunAndDebugError {
    InstallFailed(bossy::Error),
    DeviceLocked,
    PromptFailed(std::io::Error),
    DebugFailed(bossy::Error),
}

impl Reportable for RunAndDebugError {
    fn report(&self) -> Report {
        match self {
            Self::InstallFailed(err) => Report::error("Failed to install app on device", err),
            Self::DeviceLocked => Report::action_request(
                "Device is locked, so the app can't be deployed.",
                "Please unlock the device and try again!",
            ),
            Self::PromptFailed(err) => Report::error("Failed to prompt for continuation", err),
            Self::DebugFailed(err) => Report::error("Failed to debug app on device", err),
        }
    }
}

fn get_stdout_from_err(result: &bossy::Result<()>) -> Option<&str> {
    if let Err(err) = result {
        if let Some(stdout) = err.stdout_str() {
            match stdout {
                Ok(stdout) => return Some(stdout),
                Err(utf8_err) => {
                    log::error!("`ios-deploy` output wasn't valid utf-8: {}", utf8_err)
                }
            }
        } else {
            log::error!("`ios-deploy`'s output was empty");
        }
    }
    None
}

fn install(
    config: &Config,
    env: &Env,
    wrapper: &TextWrapper,
    non_interactive: opts::NonInteractive,
    id: &str,
) -> Result<(), RunAndDebugError> {
    loop {
        println!("Installing app on device...");
        let result = bossy::Command::pure("ios-deploy")
            .with_env_vars(env.explicit_env())
            .with_parsed_args("--debug --justlaunch")
            .with_args(&["--id", id])
            .with_arg("--bundle")
            .with_arg(&config.app_path())
            .with_arg("--json")
            .with_args(if non_interactive.yes() {
                Some("--noninteractive")
            } else {
                None
            })
            // This tool can apparently install over wifi, but not debug over
            // wifi... so if your device is connected over wifi (even if it's
            // wired as well) and we're using the `--debug` flag, then
            // launching will fail unless we also specify the `--no-wifi` flag
            // to keep it from trying that.
            .with_arg("--no-wifi")
            .run_and_wait_for_output()
            .map(|_| ());
        if let Some(stdout) = get_stdout_from_err(&result) {
            if Event::parse_list(stdout)
                .into_iter()
                .flat_map(|event| event.error().map(|(code, _)| code))
                .any(error::locked)
            {
                let locked = RunAndDebugError::DeviceLocked;
                if non_interactive.yes() {
                    break Err(locked);
                } else {
                    locked.report().print(wrapper);
                    prompt::minimal(
                        "Hit Enter once you've unlocked the device (or Ctrl+C to cancel)",
                    )
                    .map_err(RunAndDebugError::PromptFailed)?;
                    println!("Thanks!");
                    continue;
                }
            }
        }
        break result.map_err(RunAndDebugError::InstallFailed);
    }
}

pub fn run_and_debug(
    config: &Config,
    env: &Env,
    wrapper: &TextWrapper,
    non_interactive: opts::NonInteractive,
    id: &str,
) -> Result<(), RunAndDebugError> {
    install(config, env, wrapper, non_interactive, id)?;
    println!("Launching app on device...");
    bossy::Command::pure("ios-deploy")
        .with_env_vars(env.explicit_env())
        .with_arg("--noinstall")
        .with_args(&["--id", id])
        .with_arg("--bundle")
        .with_arg(&config.app_path())
        .with_arg("--json")
        .with_args(if non_interactive.yes() {
            Some("--noninteractive")
        } else {
            None
        })
        .with_arg("--no-wifi")
        .run_and_wait()
        .map(|_| ())
        .map_err(RunAndDebugError::DebugFailed)
}
