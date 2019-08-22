use clap::{Arg, ArgMatches};
use ginit::target::Profile;

pub fn take_a_list<'a, 'b>(arg: Arg<'a, 'b>, values: &'a [&'a str]) -> Arg<'a, 'b> {
    arg.possible_values(values)
        .multiple(true)
        .value_delimiter(" ")
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
