#![forbid(unsafe_code)]

mod cli;
mod init;
mod plugins;
mod util;

use self::{cli::*, plugins::PluginMap};
use ginit::{
    config::{RequiredUmbrella, Umbrella},
    core::{
        config::{DefaultConfigTrait, DefaultShared, RequiredConfigTrait, RequiredShared},
        opts::{Interactivity, NoiseLevel},
        util::{init_text_wrapper, TextWrapper},
    },
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
    let mut plugins = PluginMap::new();
    plugins.load("android").map_err(NonZeroExit::display)?;
    plugins.load("brainium").map_err(NonZeroExit::display)?;
    plugins.load("ios").map_err(NonZeroExit::display)?;
    let subcommands = plugins.subcommands();
    let steps = subcommands
        .iter()
        .map(|subcommand| subcommand.name)
        .collect::<Vec<_>>();
    let input = Input::parse(&plugins, app(&steps, &subcommands)).map_err(NonZeroExit::Clap)?;
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
    let plugins = plugins.configure(&config).map_err(NonZeroExit::display)?;
    let noise_level = input.noise_level;
    match input.command {
        Command::Init(command) => command
            .exec(&config, &plugins)
            .map_err(NonZeroExit::display),
        Command::Plugin { plugin_name, input } => {
            // since clap will outright reject any subcommands that aren't
            // actually part of the CLI, this get should always succeed...
            match input {
                Some(Some(input)) => plugins
                    .get(&plugin_name)
                    .unwrap()
                    .exec(input, noise_level)
                    .map_err(NonZeroExit::display),
                Some(None) => unimplemented!(),
                None => unimplemented!(),
            }
        }
    }
}

fn main() {
    let wrapper = match init_text_wrapper() {
        Ok(wrapper) => wrapper,
        Err(err) => {
            NonZeroExit::display(format!("Failed to init text wrapper: {}", err)).do_the_thing(None)
        }
    };
    if let Err(non_zero_exit) = inner(&wrapper) {
        non_zero_exit.do_the_thing(Some(wrapper))
    }
}
