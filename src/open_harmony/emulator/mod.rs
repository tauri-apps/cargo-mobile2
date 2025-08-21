mod hvd_list;

use std::{fmt::Display, path::PathBuf};

use duct::Handle;
pub use hvd_list::hvd_list;
use serde::Deserialize;

use super::env::Env;
use crate::{env::ExplicitEnv, DuctExpressionExt};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct Emulator {
    name: String,
    abi: String,
    path: PathBuf,
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
        let path = self.path.parent().unwrap().to_owned();
        // this is NOT the same as env.ohos_home()
        let image_root = dirs::home_dir()
            .unwrap()
            .join("Library")
            .join("Huawei")
            .join("Sdk");
        let emulator_path = if cfg!(target_os = "macos") {
            "/Applications/DevEco-Studio.app/Contents/tools/emulator/Emulator"
        } else {
            // TODO: windows paths
            "emulator.exe"
        };
        duct::cmd(emulator_path, ["-hvd", &self.name])
            .before_spawn(move |cmd| {
                cmd.arg("-path")
                    .arg(&path)
                    .arg("-imageRoot")
                    .arg(&image_root);
                Ok(())
            })
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
