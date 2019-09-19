#![forbid(unsafe_code)]

pub mod config;
pub mod env;
pub mod opts;
pub mod target;
pub mod templating;
pub mod util;

pub mod exports {
    pub use bicycle;
    pub use into_result;
}

use std::fmt::{Debug, Display};

pub static NAME: &'static str = "ginit";

pub trait Plugin: Debug {
    const NAME: &'static str;

    type Config: config::ConfigTrait;
    type Env: util::pure_command::ExplicitEnv;
    type InitError: Debug + Display;

    fn init(
        _config: &<Self as Plugin>::Config,
        _bike: &bicycle::Bicycle,
    ) -> Result<(), Self::InitError> {
        Ok(())
    }
}

pub trait TargetPlugin<'a>: Plugin {
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
