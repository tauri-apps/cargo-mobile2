use crate::{
    opts,
    target::TargetTrait,
    util::{init_text_wrapper, TextWrapper},
};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use colored::*;
use std::fmt::{Debug, Display};

#[macro_export]
macro_rules! cli_app {
    ($name:expr) => {
        $crate::util::cli::barebones_app(
            $name,
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_AUTHORS"),
            env!("CARGO_PKG_DESCRIPTION"),
            Some($crate::util::cli::take_init_subcommand()),
        )
    };
}

#[macro_export]
macro_rules! cli_app_custom_init {
    ($name:expr, $init:expr) => {
        $crate::util::cli::barebones_app(
            $name,
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_AUTHORS"),
            env!("CARGO_PKG_DESCRIPTION"),
            Some($init),
        )
    };
}

pub fn barebones_app<'a, 'b>(
    name: &'a str,
    version: &'a str,
    author: &'a str,
    about: &'a str,
    init: Option<App<'a, 'b>>,
) -> App<'a, 'b> {
    let mut app = App::new(name)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .version(version)
        .author(author)
        .about(about)
        .arg(take_noise_level())
        .arg(take_interactivity());
    if let Some(init) = init {
        app = app.subcommand(init.display_order(0));
    }
    app
}

pub fn take_noise_level<'a, 'b>() -> Arg<'a, 'b> {
    Arg::from_usage("-v, --verbose 'Make life louder'")
        .global(true)
        .multiple(true)
}

pub fn take_interactivity<'a, 'b>() -> Arg<'a, 'b> {
    Arg::from_usage("--non-interactive 'Go with the flow'").global(true)
}

pub fn take_init_subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("init")
        .about("Creates a new project in the current working directory")
        .arg_from_usage("--force 'Clobber files with no remorse'")
}

pub fn take_a_list<'a, 'b>(arg: Arg<'a, 'b>, values: &'a [&'a str]) -> Arg<'a, 'b> {
    arg.possible_values(values)
        .multiple(true)
        .value_delimiter(" ")
}

pub fn take_a_target_list<'a, 'b, T: TargetTrait<'a>>(targets: &'a [&'a str]) -> Arg<'a, 'b> {
    take_a_list(Arg::with_name("TARGETS"), targets).default_value(T::DEFAULT_KEY)
}

pub fn get_matches<'a, 'b>(
    app: App<'a, 'b>,
    name: impl AsRef<str>,
) -> clap::Result<ArgMatches<'a>> {
    app.get_matches_from_safe({
        let mut args: Vec<String> = std::env::args().collect();
        // Running this as a cargo subcommand gives us our name as an argument,
        // so let's just discard that...
        if args.get(1).map(String::as_str) == Some(name.as_ref()) {
            args.remove(1);
        }
        args
    })
}

pub fn get_matches_and_parse<C: CommandTrait>(
    app: App<'_, '_>,
    name: &str,
) -> clap::Result<Input<C>> {
    get_matches(app, name).map(|matches| Input::parse(&matches))
}

pub fn parse_barebones_app(matches: &ArgMatches<'_>) -> opts::Clobbering {
    let subcommand = matches.subcommand.as_ref().unwrap(); // clap makes sure we got a subcommand
    match subcommand.name.as_str() {
        "init" => parse_clobbering(&subcommand.matches),
        _ => panic!(
            "Called `parse_barebones_app` on an app which had subcommands other than \"init\"!"
        ), // ...and this is the only possible subcommand
    }
}

pub fn parse_noise_level(matches: &ArgMatches<'_>) -> opts::NoiseLevel {
    match matches.occurrences_of("verbose") {
        0 => opts::NoiseLevel::Polite,
        1 => opts::NoiseLevel::LoudAndProud,
        _ => opts::NoiseLevel::FranklyQuitePedantic,
    }
}

pub fn parse_interactivity(matches: &ArgMatches<'_>) -> opts::Interactivity {
    if matches.is_present("non-interactive") {
        opts::Interactivity::None
    } else {
        opts::Interactivity::Full
    }
}

pub fn parse_clobbering(matches: &ArgMatches<'_>) -> opts::Clobbering {
    if matches.is_present("force") {
        opts::Clobbering::Allow
    } else {
        opts::Clobbering::Forbid
    }
}

pub fn parse_targets(matches: &ArgMatches<'_>) -> Vec<String> {
    matches
        .values_of("TARGETS")
        .map(|vals| vals.map(Into::into).collect())
        .unwrap_or_default()
}

pub fn parse_profile(matches: &ArgMatches<'_>) -> opts::Profile {
    if matches.is_present("release") {
        opts::Profile::Release
    } else {
        opts::Profile::Debug
    }
}

#[derive(Debug)]
pub struct Input<C: CommandTrait> {
    pub noise_level: opts::NoiseLevel,
    pub interactivity: opts::Interactivity,
    pub command: C,
}

impl<C: CommandTrait> Input<C> {
    pub fn parse(matches: &ArgMatches<'_>) -> Self {
        Self {
            noise_level: parse_noise_level(matches),
            interactivity: parse_interactivity(matches),
            command: C::parse(matches),
        }
    }
}

pub trait CommandTrait: Debug {
    fn parse(matches: &ArgMatches<'_>) -> Self;
}

#[derive(Debug)]
pub struct InitOnly {
    pub clobbering: opts::Clobbering,
}

impl CommandTrait for InitOnly {
    fn parse(matches: &ArgMatches<'_>) -> Self {
        let subcommand = matches.subcommand.as_ref().unwrap(); // clap makes sure we got a subcommand
        let clobbering = match subcommand.name.as_str() {
            "init" => parse_clobbering(&subcommand.matches),
            _ => panic!(
                "Used `InitOnly::parse` on an app which had subcommands other than \"init\"!"
            ), // ...and this is the only possible subcommand
        };
        Self { clobbering }
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

    pub fn main(inner: impl FnOnce(&TextWrapper) -> Result<(), Self>) {
        let wrapper = match init_text_wrapper() {
            Ok(wrapper) => wrapper,
            Err(err) => {
                Self::display(format!("Failed to init text wrapper: {}", err)).do_the_thing(None)
            }
        };
        if let Err(non_zero_exit) = inner(&wrapper) {
            non_zero_exit.do_the_thing(Some(wrapper))
        }
    }
}
