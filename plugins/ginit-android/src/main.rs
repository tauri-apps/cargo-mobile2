mod adb;
mod config;
mod device;
mod env;
mod exec;
mod ndk;
mod project;
mod target;

use ginit_core::{
    cli::*, config::ConfigTrait as _, ipc, opts, templating, PluginTrait, TargetPluginTrait,
};

#[derive(Debug, Default)]
pub struct Android {
    config: Option<config::Config>,
}

impl PluginTrait for Android {
    const NAME: &'static str = "android";
    const DESCRIPTION: &'static str = "Tools for Android";

    type Config = config::Config;
    fn update_config(&mut self, config: Self::Config) {
        self.config = Some(config);
    }

    fn cli(&mut self) -> Option<Cli> {
        Some(
            Cli::default()
                .with_target_info::<Self>()
                .with_command(
                    Command::new("check", "Checks if code compiles for target(s)")
                        .with_arg(Arg::TargetList),
                )
                .with_command(
                    Command::new("build", "Builds dynamic libraries for target(s)")
                        .with_arg(Arg::TargetList)
                        .with_arg(Arg::Release),
                )
                .with_command(Command::new("run", "Deploys APK to a device").with_arg(Arg::Release))
                .with_command(Command::new(
                    "st",
                    "Displays a detailed stacktrace for a device",
                ))
                .with_command(Command::new("list", "Lists connected devices")),
        )
    }

    type InitError = project::Error;
    fn init(&mut self, _clobbering: opts::Clobbering) -> Result<(), Self::InitError> {
        let config = self.config();
        let bike = templating::init(Some(config.shared()));
        project::generate(config, &bike)
    }

    type ExecError = exec::Error;
    fn exec(
        &mut self,
        input: CliInput,
        noise_level: opts::NoiseLevel,
    ) -> Result<(), Self::ExecError> {
        exec::exec(self.config(), input, noise_level)
    }
}

impl<'a> TargetPluginTrait<'a> for Android {
    type Target = target::Target<'a>;
}

impl Android {
    fn config(&self) -> &<Self as PluginTrait>::Config {
        self.config.as_ref().unwrap()
    }
}

fn main() {
    ipc::listen(&mut Android::default()).expect("uh-oh");
}
