mod config_gen;
mod init;
mod plugin;
mod steps;

use crate::plugin::Map as PluginMap;
use ginit_core::{
    exports::once_cell_regex::regex,
    opts,
    util::cli::{self, NonZeroExit},
    NAME,
};
use structopt::clap::{self, App, AppSettings, Arg, ArgMatches, SubCommand};

fn app<'a>(
    steps: &'a [&'a str],
    subcommands: impl Iterator<Item = &'a (&'a str, &'a str)>,
) -> App<'a, 'a> {
    let mut app = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .settings(cli::SETTINGS)
        .arg(
            Arg::from_usage("-v, --verbose 'Make life louder'")
                .global(true)
                .multiple(true),
        )
        .arg(Arg::from_usage("--non-interactive 'Go with the flow'").global(true))
        .subcommand(init::app(steps));
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
    Plugin { name: String, args: Vec<String> },
}

impl Command {
    fn parse(matches: ArgMatches<'_>) -> Self {
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

fn forward_help(result: clap::Result<Input>) -> Result<Input, NonZeroExit> {
    result.or_else(|err| match err.kind {
        clap::ErrorKind::HelpDisplayed => {
            // TODO: this is silly...
            let command_re = regex!(r#"USAGE:\s+cargo-ginit (.*) \[FLAGS\]"#);
            if let Some(name) = command_re.captures_iter(&err.message).next().map(|caps| {
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
        _ => Err(NonZeroExit::Clap(err)),
    })
}

fn main() {
    NonZeroExit::main(|wrapper| {
        let plugins = {
            let mut plugins = PluginMap::default();
            plugins.add("brainium").map_err(NonZeroExit::display)?;
            plugins.add("android").map_err(NonZeroExit::display)?;
            plugins.add("ios").map_err(NonZeroExit::display)?;
            plugins
        };
        let (steps, subcommands): (Vec<_>, Vec<_>) = plugins
            .iter()
            .map(|plugin| (plugin.name(), (plugin.name(), plugin.description())))
            .unzip();
        let input = forward_help(
            app(&steps, subcommands.iter())
                .get_matches_from_safe(cli::get_args(NAME))
                .map(Input::parse),
        )?;
        cli::init_logging(input.noise_level);
        log::info!("received input {:#?}", input);
        match input.command {
            Command::Init(command) => init::exec(
                input.noise_level,
                input.interactivity,
                command,
                &plugins,
                wrapper,
            )
            .map_err(NonZeroExit::display),
            Command::Plugin { name, args } => plugins
                .get(&name)
                .ok_or_else(|| {
                    NonZeroExit::display(format!("Subcommand {:?} didn't match any plugins.", name))
                })?
                .run_and_wait(input.noise_level, input.interactivity, args)
                .map_err(NonZeroExit::display),
        }
    })
}
