mod config;
mod init;
mod plugin;
mod steps;

use crate::{config::RequiredUmbrella, plugin::Map as PluginMap};
use ginit_core::{
    cli_app_custom_init,
    config::{
        shared::{DefaultShared, RequiredShared},
        umbrella::Umbrella,
        DefaultConfigTrait, RequiredConfigTrait,
    },
    exports::clap::{App, AppSettings, ArgMatches, SubCommand},
    opts::{Interactivity, NoiseLevel},
    util::{
        cli::{self, Input, NonZeroExit},
        TextWrapper,
    },
    NAME,
};
use regex::Regex;

fn app<'a>(
    steps: &'a [&'a str],
    subcommands: impl Iterator<Item = &'a (&'a str, &'a str)>,
) -> App<'a, 'a> {
    let mut app = cli_app_custom_init!(NAME, init::app(steps));
    for (order, (name, description)) in subcommands.enumerate() {
        app = app.subcommand(
            SubCommand::with_name(name)
                .setting(AppSettings::AllowExternalSubcommands)
                .about(*description)
                .display_order(order + 1),
        );
    }
    app
}

#[derive(Debug)]
enum Command {
    Init(init::Command),
    Plugin { name: String, args: Vec<String> },
}

impl cli::CommandTrait for Command {
    fn parse(matches: &ArgMatches<'_>) -> Self {
        let subcommand = matches.subcommand.as_ref().unwrap(); // clap makes sure we got a subcommand
        match subcommand.name.as_str() {
            "init" => Self::Init(init::Command::parse(&subcommand.matches)),
            _ => Self::Plugin {
                name: subcommand.name.to_owned(),
                args: subcommand
                    .matches
                    .subcommand
                    .as_ref()
                    .map(|sub_subcommand| {
                        let mut args = vec![sub_subcommand.name.to_owned()];
                        if let Some(values) = sub_subcommand.matches.values_of("") {
                            args.extend(values.map(|arg| arg.to_owned()));
                        }
                        args
                    })
                    .unwrap_or_default(),
            },
        }
    }
}

fn forward_help(result: clap::Result<Input<Command>>) -> Result<Input<Command>, NonZeroExit> {
    result.or_else(|err| match err.kind {
        clap::ErrorKind::HelpDisplayed => {
            lazy_static::lazy_static! {
                // TODO: this is silly...
                static ref COMMAND_RE: Regex = Regex::new(r#"USAGE:\s+cargo-ginit (.*) \[FLAGS\]"#).unwrap();
            }
            if let Some(name) = COMMAND_RE.captures_iter(&err.message).next().map(|caps| {
                assert_eq!(caps.len(), 2);
                caps.get(1).unwrap().as_str().to_owned()
            }) {
                Ok(Input {
                    noise_level: Default::default(),
                    interactivity: Default::default(),
                    command: Command::Plugin {
                        name,
                        args: vec!["help".to_owned()],
                    },
                })
            } else {
                Err(NonZeroExit::Clap(err))
            }
        }
        _ => Err(NonZeroExit::Clap(err))
    })
}

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
    plugins.add("brainium").map_err(NonZeroExit::display)?;
    plugins.add("ios").map_err(NonZeroExit::display)?;
    let (steps, subcommands): (Vec<_>, Vec<_>) = plugins
        .iter()
        .map(|plugin| (plugin.name(), (plugin.name(), plugin.description())))
        .unzip();
    let input = forward_help(cli::get_matches_and_parse(
        app(&steps, subcommands.iter()),
        NAME,
    ))?;
    init_logging(input.noise_level);
    log::info!("received input {:#?}", input);
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
    match input.command {
        Command::Init(command) => init::exec(
            Input {
                noise_level: input.noise_level,
                interactivity: input.interactivity,
                command,
            },
            &plugins,
        )
        .map_err(NonZeroExit::display),
        Command::Plugin { name, args } => plugins
            .get(&name)
            .ok_or_else(|| {
                NonZeroExit::display(format!("Subcommand {:?} didn't match any plugins.", name))
            })?
            .run(input.noise_level, input.interactivity, args)
            .map_err(NonZeroExit::display)
            .map(drop),
    }
}

fn main() {
    NonZeroExit::main(inner)
}
