#![forbid(unsafe_code)]

mod cli;
mod init;

use self::cli::*;
use ginit::{
    config::RequiredUmbrella,
    core::{
        config::{
            shared::{DefaultShared, RequiredShared},
            umbrella::Umbrella,
            DefaultConfigTrait, RequiredConfigTrait,
        },
        opts::{Interactivity, NoiseLevel},
        util::{cli::NonZeroExit, TextWrapper},
    },
    plugin::Map as PluginMap,
};

fn init_logging(noise_level: NoiseLevel) {
    use env_logger::{Builder, Env};
    let default_level = match noise_level {
        NoiseLevel::Polite => "warn",
        NoiseLevel::LoudAndProud => "ginit=info",
        NoiseLevel::FranklyQuitePedantic => "info",
    };
    let env = Env::default().default_filter_or(default_level);
    Builder::from_env(env).init();
}

fn inner(wrapper: &TextWrapper) -> Result<(), NonZeroExit> {
    let mut plugins = PluginMap::default();
    plugins.add("android").map_err(NonZeroExit::display)?;
    // plugins.add("brainium").map_err(NonZeroExit::display)?;
    // plugins.add("ios").map_err(NonZeroExit::display)?;
    let (steps, subcommands): (Vec<_>, Vec<_>) = plugins
        .iter()
        .map(|plugin| (plugin.name(), (plugin.name(), plugin.description())))
        .unzip();
    let input = Input::parse(app(&steps, subcommands.iter())).map_err(NonZeroExit::Clap)?;
    init_logging(input.noise_level);
    let config = Umbrella::load(".").map_err(NonZeroExit::display)?.map_or_else(
        || {
            // let old_bike = templating::init(None);
            let required_config = match input.interactivity {
                Interactivity::Full => {
                    let shared= RequiredShared::prompt(&wrapper).map_err(NonZeroExit::display)?;
                    let mut umbrella = RequiredUmbrella::new(shared);
                    // for plugin in unimplemented!() {
                    //     unimplemented!()
                    // }
                    umbrella
                }
                Interactivity::None => {
                    let shared = DefaultShared::detect().map_err(NonZeroExit::display).and_then(|defaults| defaults.upgrade().map_err(NonZeroExit::display))?;
                    let mut umbrella = RequiredUmbrella::new(shared);
                    // for plugin in unimplemented!() {
                    //     unimplemented!()
                    // }
                    umbrella
                }
            };
            required_config.write(".").map_err(NonZeroExit::display)?;
            if let Some(config) = Umbrella::load(".").map_err(NonZeroExit::display)? {
                Ok(config)
            } else {
                Err(NonZeroExit::display("Developer error: no config found even after doing a successful `interactive_config_gen`!"))
            }
        },
        |config| {
            Ok(config)
        },
    )?;
    let noise_level = input.noise_level;
    let interactivity = input.interactivity;
    match input.command {
        Command::Init(command) => command
            .exec(&plugins, noise_level, interactivity)
            .map_err(NonZeroExit::display),
        Command::Plugin { name, args } => plugins
            .get(&name)
            .ok_or_else(|| {
                NonZeroExit::display(format!("Subcommand {:?} didn't match any plugins.", name))
            })?
            .run(noise_level, interactivity, args)
            .map_err(NonZeroExit::display)
            .map(drop),
    }
}

fn main() {
    NonZeroExit::main(inner)
}
