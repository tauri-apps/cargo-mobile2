use crate::init::{self, InitCommand};
use clap::{App, AppSettings, Arg, SubCommand};
use ginit::core::{
    opts,
    util::cli::{get_matches, parse_noise_level, take_noise_level},
    NAME,
};

pub fn app<'a>(
    steps: &'a [&'a str],
    subcommands: impl Iterator<Item = &'a (&'a str, &'a str)>,
) -> App<'a, 'a> {
    let mut app = App::new(NAME)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(take_noise_level())
        .arg(Arg::from_usage("--non-interactive 'Go with the flow'").global(true))
        .subcommand(init::subcommand(steps).display_order(0));
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
pub struct Input {
    pub noise_level: opts::NoiseLevel,
    pub interactivity: opts::Interactivity,
    pub command: Command,
}

impl Input {
    pub fn parse(app: App<'_, '_>) -> clap::Result<Self> {
        get_matches(app, NAME).map(|matches| Self {
            noise_level: parse_noise_level(&matches),
            interactivity: if matches.is_present("non-interactive") {
                opts::Interactivity::None
            } else {
                opts::Interactivity::Full
            },
            command: Command::parse(*matches.subcommand.unwrap()), // claps makes sure we got a subcommand
        })
    }
}

#[derive(Debug)]
pub enum Command {
    Init(InitCommand),
    Plugin { name: String, args: Vec<String> },
}

impl Command {
    pub fn parse(command: SubCommand<'_>) -> Self {
        let command_name = command.name.as_str();
        if command_name == "init" {
            Self::Init(InitCommand::parse(command.matches))
        } else {
            Self::Plugin {
                name: command_name.to_owned(),
                args: command
                    .matches
                    .subcommand
                    .and_then(|subcommand| {
                        subcommand.matches.values_of("").map(|values| {
                            let mut args = vec![subcommand.name.to_owned()];
                            args.extend(values.map(|arg| arg.to_owned()));
                            args
                        })
                    })
                    .unwrap_or_default(),
            }
        }
    }
}
