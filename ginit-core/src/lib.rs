#![forbid(unsafe_code)]

pub mod cli;
pub mod config;
pub mod env;
pub mod ipc;
pub mod opts;
pub mod protocol;
pub mod target;
pub mod templating;
pub mod util;

pub mod exports {
    pub use bicycle;
    pub use into_result;
}

use std::fmt::{Debug, Display};

pub static NAME: &'static str = "ginit";

pub trait PluginTrait: Debug {
    const NAME: &'static str;
    const DESCRIPTION: &'static str;

    type Env: util::pure_command::ExplicitEnv;

    type Config: config::ConfigTrait;
    fn update_config(&mut self, config: Self::Config);

    fn cli(&mut self) -> Option<cli::Cli> {
        None
    }

    type InitError: Debug + Display;
    fn init(&mut self) -> Result<(), Self::InitError> {
        Ok(())
    }

    type ExecError: Debug + Display;
    fn exec(
        &mut self,
        _input: cli::CliInput,
        _noise_level: opts::NoiseLevel,
    ) -> Result<(), Self::ExecError> {
        Ok(())
    }
}

pub trait TargetPluginTrait<'a>: PluginTrait {
    // type CargoConfigError: Debug + Display;
    type Target: target::TargetTrait<'a> + 'a;

    fn install_toolchains() -> into_result::command::CommandResult<()> {
        use target::TargetTrait as _;
        for target in Self::Target::all().values() {
            util::rustup_add(target.triple())?;
        }
        Ok(())
    }

    // fn cargo_config(
    //     config: &<Self as Plugin>::Config,
    //     env: &<Self as Plugin>::Env,
    // ) -> Result<Vec<target::TargetCargoConfig>, Self::CargoConfigError>;
}
