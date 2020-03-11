mod config_gen;
mod init;
mod plugin;
mod steps;

use crate::plugin::Map as PluginMap;
use ginit_core::{
    config::umbrella::Umbrella,
    opts,
    util::{
        self,
        cli::{self, NonZeroExit},
    },
    NAME,
};
use structopt::clap::{self, App, AppSettings, Arg, ArgMatches, SubCommand};

fn app<'a>(
    steps: &'a [&'a str],
    subcommands: impl Iterator<Item = &'a (&'a str, &'a str)>,
) -> App<'a, 'a> {
    let mut app = App::new(env!("CARGO_PKG_NAME"))
        .bin_name("cargo ginit")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .global_settings(cli::GLOBAL_SETTINGS)
        .settings(cli::SETTINGS)
        .setting(AppSettings::AllowExternalSubcommands)
        .arg(
            Arg::from_usage("-v, --verbose 'Make life louder'")
                .global(true)
                .multiple(true),
        )
        .arg(Arg::from_usage("--non-interactive 'Go with the flow'").global(true))
        .subcommand(init::app(steps))
        .subcommand(SubCommand::with_name("open").about("Open in default code editor"));
    for (name, description) in subcommands {
        app = app.subcommand(
            SubCommand::with_name(name)
                .setting(AppSettings::AllowExternalSubcommands)
                .about(*description),
        );
    }
    app
}

#[derive(Debug)]
struct Input {
    noise_level: opts::NoiseLevel,
    interactivity: opts::Interactivity,
    command: Command,
}

impl Input {
    fn parse(matches: ArgMatches<'_>) -> Self {
        Self {
            noise_level: cli::noise_level_from_occurrences(matches.occurrences_of("verbose")),
            interactivity: if std::env::var("CI").ok().filter(|s| s == "true").is_some() {
                log::info!(
                    "env var `CI` is set to `true`; automatically running in non-interactive mode"
                );
                opts::Interactivity::None
            } else {
                cli::interactivity_from_presence(matches.is_present("non-interactive"))
            },
            command: Command::parse(matches),
        }
    }
}

#[derive(Debug)]
enum Command {
    Init(init::Command),
    Open,
    Plugin { name: String, args: Vec<String> },
}

impl Command {
    fn parse(matches: ArgMatches<'_>) -> Self {
        let subcommand = matches.subcommand.as_ref().unwrap(); // clap makes sure we got a subcommand
        match subcommand.name.as_str() {
            "init" => Self::Init(init::Command::parse(&subcommand.matches)),
            "open" => Self::Open,
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
                    .filter(|args| !args.is_empty())
                    .unwrap_or_else(|| vec!["help".to_owned()]),
            },
        }
    }
}

fn forward_help<'a>(
    result: Result<Input, (clap::Error, impl Iterator<Item = &'a String>, &PluginMap)>,
) -> Result<Input, NonZeroExit> {
    result.or_else(|(err, args, plugins)| match err.kind {
        clap::ErrorKind::HelpDisplayed => {
            if let Some(name) = args
                // Skip the binary path
                .skip(1)
                // Get the first thing that's not a flag
                .find(|arg| !arg.starts_with("-"))
                // We only proceed if it's a plugin
                .filter(|name| plugins.get(name).is_some())
            {
                Ok(Input {
                    noise_level: Default::default(),
                    interactivity: Default::default(),
                    command: Command::Plugin {
                        name: name.to_owned(),
                        args: vec!["help".to_owned()],
                    },
                })
            } else {
                Err(NonZeroExit::Clap(err))
            }
        }
        _ => Err(NonZeroExit::Clap(err)),
    })
}

fn main() {
    NonZeroExit::main(|wrapper| {
        let plugins = Umbrella::load(".")
            .map_err(NonZeroExit::display)?
            .map(|umbrella| PluginMap::from_shared(umbrella.shared()))
            .transpose()
            .map_err(NonZeroExit::display)?
            .unwrap_or_default();
        let (steps, subcommands): (Vec<_>, Vec<_>) = plugins
            .iter()
            .map(|plugin| (plugin.name(), (plugin.name(), plugin.description())))
            .unzip();
        let args = cli::get_args(NAME);
        let input = forward_help(
            app(&steps, subcommands.iter())
                .get_matches_from_safe(&args)
                .map(Input::parse)
                .map_err(|err| (err, args.iter(), &plugins)),
        )?;
        cli::init_logging(input.noise_level);
        log::info!("received input: {:#?}", input);
        match input.command {
            Command::Init(command) => {
                init::exec(input.noise_level, input.interactivity, command, wrapper)
                    .map_err(NonZeroExit::display)
            }
            Command::Open => util::open_in_editor(".").map_err(NonZeroExit::display),
            Command::Plugin { name, args } => plugins
                .get(&name)
                .ok_or_else(|| {
                    NonZeroExit::display(format!("Subcommand {:?} didn't match any plugins.", name))
                })?
                .command(input.noise_level, input.interactivity, args)
                .run_and_wait()
                .map(|_| ())
                .map_err(NonZeroExit::display),
        }
    })
}
