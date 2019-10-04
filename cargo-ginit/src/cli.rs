use crate::{
    init::{self, InitCommand},
    plugins::PluginMap,
    util,
};
use clap::{App, AppSettings, Arg, SubCommand};
use colored::*;
use ginit::core::{
    cli,
    // init::config_gen::{DefaultConfig, RequiredConfig},
    opts::{Interactivity, NoiseLevel},
    util::TextWrapper,
    NAME,
};
use std::fmt::Display;

pub fn app<'a>(steps: &'a [&'a str], subcommands: &'a [util::CliInfo<'a>]) -> App<'a, 'a> {
    let mut app = App::new(NAME)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::from_usage("-v, --verbose 'Make life louder'")
                .global(true)
                .multiple(true),
        )
        .arg(Arg::from_usage("--non-interactive 'Go with the flow'").global(true))
        .subcommand(init::subcommand(steps).display_order(0));
    for (order, subcommand) in subcommands.iter().enumerate() {
        app = app.subcommand(subcommand.render().display_order(order + 1));
    }
    app
}

#[derive(Debug)]
pub struct Input {
    pub noise_level: NoiseLevel,
    pub interactivity: Interactivity,
    pub command: Command,
}

impl Input {
    pub fn parse<State>(plugins: &PluginMap<State>, app: App<'_, '_>) -> clap::Result<Self> {
        let args = {
            let mut args: Vec<String> = std::env::args().collect();
            // Running this as a cargo subcommand gives us our name as an argument,
            // so let's just discard that...
            if args.get(1).map(String::as_str) == Some(NAME) {
                args.remove(1);
            }
            args
        };
        app.get_matches_from_safe(args).map(|matches| Self {
            noise_level: match matches.occurrences_of("verbose") {
                0 => NoiseLevel::Polite,
                1 => NoiseLevel::LoudAndProud,
                _ => NoiseLevel::FranklyQuitePedantic,
            },
            interactivity: if matches.is_present("non-interactive") {
                Interactivity::None
            } else {
                Interactivity::Full
            },
            command: Command::parse(plugins, *matches.subcommand.unwrap()), // claps makes sure we got a subcommand
        })
    }
}

#[derive(Debug)]
pub enum Command {
    Init(InitCommand),
    Plugin {
        plugin_name: String,
        input: Option<Option<cli::CliInput>>,
    },
}

impl Command {
    pub fn parse<State>(plugins: &PluginMap<State>, command: SubCommand<'_>) -> Self {
        let command_name = command.name.as_str();
        if command_name == "init" {
            Self::Init(InitCommand::parse(command.matches))
        } else {
            Self::Plugin {
                plugin_name: command_name.to_owned(),
                input: plugins
                    .get(command_name)
                    .unwrap()
                    .cli()
                    .map(|cli| util::parse_input(cli, command.matches)),
            }
        }
    }
}

#[derive(Debug)]
pub enum NonZeroExit {
    Display(String),
    Clap(clap::Error),
}

impl NonZeroExit {
    pub fn display(err: impl Display) -> Self {
        Self::Display(format!("{}", err))
    }

    pub fn do_the_thing(self, wrapper: Option<TextWrapper>) -> ! {
        match self {
            Self::Display(err) => {
                eprintln!(
                    "{}",
                    if let Some(wrapper) = wrapper {
                        wrapper.fill(&err).bright_red()
                    } else {
                        err.bright_red()
                    }
                );
                std::process::exit(1)
            }
            Self::Clap(err) => err.exit(),
        }
    }
}
