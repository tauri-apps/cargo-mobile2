use crate::util;
use clap::ArgMatches;
use ginit::core::cli::{Arg, ArgInput, Cli, CliInput};

pub fn parse(cli: &Cli, matches: ArgMatches<'_>) -> Option<CliInput> {
    // clap makes sure we got a subcommand
    let subcommand = matches.subcommand.as_ref().unwrap();
    if let Some(schema) = cli
        .commands
        .iter()
        .find(|command| command.name == subcommand.name)
    {
        Some(CliInput {
            command: schema.name.clone(),
            args: schema
                .args
                .iter()
                .map(|arg| match arg {
                    Arg::Custom { name, .. } => ArgInput::Custom {
                        name: name.clone(),
                        present: subcommand.matches.is_present(&name),
                        value: subcommand.matches.value_of(&name).map(|v| v.to_owned()),
                    },
                    Arg::FromUsage { usage } => {
                        let name = util::name_from_usage(&usage);
                        ArgInput::FromUsage {
                            name: name.to_owned(),
                            present: subcommand.matches.is_present(&name),
                            value: subcommand.matches.value_of(&name).map(|v| v.to_owned()),
                        }
                    }
                    Arg::TargetList => ArgInput::TargetList {
                        targets: util::parse_targets(&subcommand.matches),
                    },
                    Arg::Release => ArgInput::Release {
                        profile: util::parse_profile(&subcommand.matches),
                    },
                })
                .collect(),
        })
    } else {
        None
    }
}
