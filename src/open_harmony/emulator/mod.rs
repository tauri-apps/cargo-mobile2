mod hvd_list;

use std::fmt::Display;

use duct::Handle;
pub use hvd_list::hvd_list;
use serde::Deserialize;

use super::env::Env;
use crate::{env::ExplicitEnv, DuctExpressionExt};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct Emulator {
    name: String,
    abi: String,
    path: String,
    #[serde(rename = "harmonyos.sdk.path")]
    image_root: String,
}

impl Display for Emulator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Emulator {
    pub fn name(&self) -> &str {
        &self.name
    }

    fn command(&self, env: &Env) -> duct::Expression {
        let emulator_path = "/Applications/DevEco-Studio.app/Contents/tools/emulator/Emulator";
        duct::cmd(
            emulator_path,
            [
                "-hvd",
                &self.name,
                "-path",
                &self.path,
                "-imageRoot",
                &self.image_root,
            ],
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
