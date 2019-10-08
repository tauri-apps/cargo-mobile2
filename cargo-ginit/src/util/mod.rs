mod input;
mod render;

pub use self::{input::parse as parse_input, render::CliInfo};

use clap::{Arg, ArgMatches};
use ginit::core::opts::Profile;

pub fn take_a_list<'a, 'b>(arg: Arg<'a, 'b>, values: &'a [&'a str]) -> Arg<'a, 'b> {
    arg.possible_values(values)
        .multiple(true)
        .value_delimiter(" ")
}

pub fn take_a_target_list<'a, 'b>(targets: &'a [&'a str], default: &'a str) -> Arg<'a, 'b> {
    take_a_list(Arg::with_name("TARGETS"), targets).default_value(default)
}

pub fn name_from_usage<'a>(usage: &'a str) -> &'a str {
    Arg::from_usage(usage).b.name
}

pub fn parse_targets(matches: &ArgMatches<'_>) -> Vec<String> {
    matches
        .values_of("TARGETS")
        .map(|vals| vals.map(Into::into).collect())
        .unwrap_or_default()
}

pub fn parse_profile(matches: &ArgMatches<'_>) -> Profile {
    if matches.is_present("release") {
        Profile::Release
    } else {
        Profile::Debug
    }
}
