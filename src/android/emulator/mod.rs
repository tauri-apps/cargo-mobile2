mod avd_list;

use std::{fmt::Display, path::PathBuf};

pub use avd_list::avd_list;

use super::env::Env;
use crate::{bossy, env::ExplicitEnv};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Emulator {
    name: String,
}

impl Display for Emulator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Emulator {
    fn new(name: String) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn start_inner(&self, env: &Env) -> bossy::Command {
        bossy::Command::impure(PathBuf::from(env.android_home()).join("emulator/emulator"))
            .with_args(["-avd", &self.name])
            .with_env_vars(env.explicit_env())
    }

    pub fn start(&self, env: &Env) -> bossy::Result<bossy::Handle> {
        self.start_inner(env).run()
    }

    pub fn start_detached(&self, env: &Env) -> bossy::Result<()> {
        self.start_inner(env).run_and_detach()
    }
}
