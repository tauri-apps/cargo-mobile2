mod config_gen;
mod init;
mod plugin;
mod steps;

use crate::plugin::Map as PluginMap;
use ginit_core::{
    cli_app_custom_init,
    exports::{
        clap::{App, AppSettings, ArgMatches, SubCommand},
        once_cell_regex::regex,
    },
    opts::NoiseLevel,
    util::cli::{self, Input, NonZeroExit},
    NAME,
};

fn app<'a>(
    steps: &'a [&'a str],
    subcommands: impl Iterator<Item = &'a (&'a str, &'a str)>,
) -> App<'a, 'a> {
    let mut app = cli_app_custom_init!(init::app(steps));
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
        let input = forward_help(cli::get_matches_and_parse(
            app(&steps, subcommands.iter()),
            NAME,
        ))?;
        init_logging(input.noise_level);
        log::info!("received input {:#?}", input);
        match input.command {
            Command::Init(command) => init::exec(
                Input {
                    noise_level: input.noise_level,
                    interactivity: input.interactivity,
                    command,
                },
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
