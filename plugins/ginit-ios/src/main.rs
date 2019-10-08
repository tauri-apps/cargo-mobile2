mod config;
mod deps;
mod device;
mod exec;
mod ios_deploy;
mod project;
mod system_profile;
mod target;
mod teams;

use ginit_core::{
    cli::*, config::ConfigTrait as _, ipc, opts, templating, PluginTrait, TargetPluginTrait,
};

#[derive(Debug, Default)]
pub struct IOS {
    config: Option<config::Config>,
}

impl PluginTrait for IOS {
    const NAME: &'static str = "ios";
    const DESCRIPTION: &'static str = "Tools for iOS";

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
                    Command::new("build", "Builds static libraries for target(s)")
                        .with_arg(Arg::TargetList)
                        .with_arg(Arg::Release),
                )
                .with_command(Command::new("run", "Deploys IPA to a device").with_arg(Arg::Release))
                .with_command(Command::new("list", "Lists connected devices"))
                .with_command(
                    Command::new(
                        "compile-lib",
                        "Compiles static lib (should only be called by Xcode!)",
                    )
                    .with_hidden(true)
                    .with_arg(Arg::from_usage(
                        "--macos 'Awkwardly special-case for macOS'",
                    ))
                    .with_arg(Arg::custom("ARCH", true, Some(1)))
                    .with_arg(Arg::Release),
                ),
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

impl<'a> TargetPluginTrait<'a> for IOS {
    type Target = target::Target<'a>;
}

impl IOS {
    fn config(&self) -> &<Self as PluginTrait>::Config {
        self.config.as_ref().unwrap()
    }
}

fn main() {
    ipc::listen(&mut IOS::default()).expect("uh-oh");
}
