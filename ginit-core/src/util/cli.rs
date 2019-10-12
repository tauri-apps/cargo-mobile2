use crate::{
    opts,
    target::TargetTrait,
    util::{init_text_wrapper, TextWrapper},
};
use clap::{App, Arg, ArgMatches};
use colored::*;
use std::fmt::Display;

pub fn take_noise_level<'a, 'b>() -> Arg<'a, 'b> {
    Arg::from_usage("-v, --verbose 'Make life louder'")
        .global(true)
        .multiple(true)
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

pub fn parse_noise_level(matches: &ArgMatches<'_>) -> opts::NoiseLevel {
    match matches.occurrences_of("verbose") {
        0 => opts::NoiseLevel::Polite,
        1 => opts::NoiseLevel::LoudAndProud,
        _ => opts::NoiseLevel::FranklyQuitePedantic,
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
