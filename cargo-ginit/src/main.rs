#![forbid(unsafe_code)]

mod cli;
mod init;
mod plugins;
mod util;

use self::{cli::*, plugins::PluginMap};
use ginit::{
    config::Umbrella,
    core::{
        // init::config_gen::{DefaultConfig, RequiredConfig},
        opts::NoiseLevel,
        templating,
        util::{init_text_wrapper, TextWrapper},
        NAME,
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
    plugins.load("android");
    let subcommands = plugins.subcommands();
    let input = Input::parse(&plugins, app(&subcommands)).map_err(NonZeroExit::Clap)?;
    init_logging(input.noise_level);
    let config = Umbrella::load(".").map_err(NonZeroExit::display)?.map_or_else(
        || {
            // let old_bike = templating::init(None);
            // let required_config = match input.interactivity {
            //     Interactivity::Full => {
            //         handle_error(&wrapper, RequiredConfig::interactive(&wrapper))
            //     }
            //     Interactivity::None => {
            //         let defaults = handle_error(&wrapper, DefaultConfig::detect());
            //         handle_error(&wrapper, defaults.upgrade())
            //     }
            // };
            // handle_error(&wrapper, required_config.write(&old_bike, "."));
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
    let plugins = plugins.configure(&config);
    let noise_level = input.noise_level;
    match input.command {
        Command::Init(command) => command.exec(&config).map_err(NonZeroExit::display),
        Command::Plugin { plugin_name, input } => {
            // since clap will outright reject any subcommands that aren't
            // actually part of the CLI, this get should always succeed...
            // TODO: noise level
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
