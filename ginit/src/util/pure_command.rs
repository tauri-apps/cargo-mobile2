use std::{ffi::OsStr, fmt::Debug, process::Command};

pub trait ExplicitEnv: Debug {
    fn explicit_env(&self) -> Vec<(&str, &OsStr)>;
}

#[derive(Debug)]
pub enum PureCommand {}

impl PureCommand {
    pub fn new(name: impl AsRef<OsStr>, env: &impl ExplicitEnv) -> Command {
        let mut command = Command::new(name);
        command.env_clear().envs(env.explicit_env());
        command
    }
}
