mod avd_list;

use std::{fmt::Display, path::PathBuf};

pub use avd_list::avd_list;
use duct::Handle;

use super::env::Env;
use crate::{env::ExplicitEnv, DuctExpressionExt};

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

    fn command(&self, env: &Env) -> duct::Expression {
        duct::cmd(
            PathBuf::from(env.android_home()).join("emulator/emulator"),
            ["-avd", &self.name],
        )
        .vars(env.explicit_env())
        .dup_stdio()
    }

    pub fn start(&self, env: &Env) -> Result<Handle, std::io::Error> {
        self.command(env).start()
    }

    pub fn start_detached(&self, env: &Env) -> Result<(), std::io::Error> {
        self.command(env).run_and_detach()?;
        Ok(())
    }
}
