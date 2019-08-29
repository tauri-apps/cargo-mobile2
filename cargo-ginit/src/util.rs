use clap::{Arg, ArgMatches};
use ginit::target::{Profile, TargetTrait};

pub fn take_a_list<'a, 'b>(arg: Arg<'a, 'b>, values: &'a [&'a str]) -> Arg<'a, 'b> {
    arg.possible_values(values)
        .multiple(true)
        .value_delimiter(" ")
}

pub fn take_a_target_list<'a, 'b, T: TargetTrait<'a>>(targets: &'a [&'a str]) -> Arg<'a, 'b> {
    take_a_list(Arg::with_name("TARGETS"), targets).default_value(T::DEFAULT_KEY)
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
