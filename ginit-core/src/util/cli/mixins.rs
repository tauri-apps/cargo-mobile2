use crate::opts;
use clap::AppSettings;
use structopt::StructOpt;

pub static SETTINGS: &'static [AppSettings] = &[
    AppSettings::ColoredHelp,
    AppSettings::SubcommandRequiredElseHelp,
    AppSettings::VersionlessSubcommands,
];

#[derive(Debug, StructOpt)]
pub struct GlobalFlags {
    #[structopt(
        short = "v",
        long = "verbose",
        about = "Make life louder",
        global = true,
        multiple = true,
        parse(from_occurrences = super::noise_level_from_occurrences),
    )]
    pub noise_level: opts::NoiseLevel,
    #[structopt(
        long = "non-interactive",
        about = "Go with the flow",
        global = true,
        parse(from_flag = super::interactivity_from_presence),
    )]
    pub interactivity: opts::Interactivity,
}

#[derive(Debug, StructOpt)]
pub struct Clobbering {
    #[structopt(
        long = "force",
        about = "Clobber files with no remorse",
        parse(from_flag = super::clobbering_from_presence),
    )]
    pub clobbering: opts::Clobbering,
}

#[derive(Debug, StructOpt)]
pub struct Profile {
    #[structopt(
        long = "release",
        about = "Compile with optimizations",
        parse(from_flag = super::profile_from_presence),
    )]
    pub profile: opts::Profile,
}
