mod android;
mod init;
mod ios;
mod util;

use self::{android::AndroidCommand, init::InitCommand, ios::IOSCommand};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use ginit::{config::Config, NAME};

fn target_lists<'a>(config: &'a Option<Config>) -> (Vec<&'a str>, Vec<&'a str>) {
    let android_targets = config
        .as_ref()
        .map(|config| {
            config
                .android()
                .targets()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let ios_targets = config
        .as_ref()
        .map(|config| {
            config
                .ios()
                .targets()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    (android_targets, ios_targets)
}

fn cli_app<'a, 'b>(android_targets: &'a [&'a str], ios_targets: &'a [&'a str]) -> App<'a, 'b> {
    App::new(NAME)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::from_usage("-v, --verbose 'Make life louder'").global(true))
        .subcommand(init::subcommand().display_order(0))
        .subcommand(android::subcommand(android_targets).display_order(1))
        .subcommand(ios::subcommand(ios_targets).display_order(1))
}

#[derive(Debug)]
struct CliInput {
    verbose: bool,
    command: Command,
}

impl CliInput {
    fn parse(matches: ArgMatches<'_>) -> Self {
        Self {
            verbose: matches.is_present("verbose"),
            command: Command::parse(*matches.subcommand.unwrap()), // claps makes sure we got a subcommand
        }
    }
}

#[derive(Debug)]
enum Command {
    Init(InitCommand),
    Android(AndroidCommand),
    IOS(IOSCommand),
}

impl Command {
    fn parse(command: SubCommand<'_>) -> Self {
        match command.name.as_str() {
            "init" => Command::Init(InitCommand::parse(command.matches)),
            "android" => Command::Android(AndroidCommand::parse(command.matches)),
            "ios" => Command::IOS(IOSCommand::parse(command.matches)),
            _ => unreachable!(), // clap will reject anything else
        }
    }
}

fn get_args() -> (Vec<String>, bool) {
    let mut raw: Vec<String> = std::env::args().collect();
    // Running this as a cargo subcommand gives us our name as an argument,
    // so let's just discard that...
    if raw.get(1).map(String::as_str) == Some(NAME) {
        raw.remove(1);
    }

    // We have to initialize logging as early as possible, so we'll do some
    // rough manual parsing.
    let verbose = {
        use ginit::util::FriendlyContains as _;
        // This will fail if multiple short flags are put together, i.e. `-vh`
        raw.as_slice().friendly_contains("--verbose") || raw.as_slice().friendly_contains("-v")
    };

    (raw, verbose)
}

fn log_init(verbose: bool) {
    use env_logger::{Builder, Env};
    let default_level = if verbose { "info" } else { "warn" };
    let env = Env::default().default_filter_or(default_level);
    Builder::from_env(env).init();
}

fn main() {
    let (args, verbose) = get_args();
    log_init(verbose);

    let config = Config::load(".").expect("failed to load config");

    let (android_targets, ios_targets) = target_lists(&config);
    let app = cli_app(&android_targets, &ios_targets);
    let input = CliInput::parse(app.get_matches_from(args));
    match input.command {
        Command::Init(command) => {
            command.exec(config.as_ref());
        }
        Command::Android(command) => {
            let config = config.as_ref().expect("ginit.toml not found");
            command.exec(config, input.verbose);
        }
        Command::IOS(command) => {
            let config = config.as_ref().expect("ginit.toml not found");
            command.exec(config, input.verbose);
        }
    }
}
