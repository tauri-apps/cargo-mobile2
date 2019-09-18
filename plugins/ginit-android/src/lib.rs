pub mod adb;
pub mod config;
pub mod device;
pub mod env;
pub mod ndk;
pub mod project;
pub mod target;

use ginit_core::{exports::bicycle, Plugin, TargetPlugin};

#[derive(Debug)]
pub struct Android;

impl Plugin for Android {
    const NAME: &'static str = "android";

    type Config = config::Config;
    type Env = env::Env;
    type InitError = project::Error;

    fn init(
        config: &<Self as Plugin>::Config,
        bike: &bicycle::Bicycle,
    ) -> Result<(), Self::InitError> {
        project::generate(config, bike)
    }
}

impl<'a> TargetPlugin<'a> for Android {
    type Target = target::Target<'a>;
}
