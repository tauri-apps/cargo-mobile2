use crate::opts;
use std::fmt::Debug;
use structopt::{clap::AppSettings, StructOpt};

pub static SETTINGS: &'static [AppSettings] = &[
    AppSettings::ColoredHelp,
    AppSettings::DeriveDisplayOrder,
    AppSettings::SubcommandRequiredElseHelp,
    AppSettings::VersionlessSubcommands,
];

pub fn bin_name(name: &str) -> String {
    format!("cargo {}", name)
}

#[derive(Clone, Copy, Debug, StructOpt)]
pub struct Clobbering {
    #[structopt(
        long = "force",
        about = "Clobber files with no remorse",
        parse(from_flag = clobbering_from_presence),
    )]
    pub clobbering: opts::Clobbering,
}

#[derive(Clone, Copy, Debug, StructOpt)]
pub struct Profile {
    #[structopt(
        long = "release",
        about = "Build with release optimizations",
        parse(from_flag = profile_from_presence),
    )]
    pub profile: opts::Profile,
}

pub fn clobbering_from_presence(present: bool) -> opts::Clobbering {
    if present {
        opts::Clobbering::Allow
    } else {
        opts::Clobbering::Forbid
    }
}

pub fn profile_from_presence(present: bool) -> opts::Profile {
    if present {
        opts::Profile::Release
    } else {
        opts::Profile::Debug
    }
}

pub fn open_in_from_presence(present: bool) -> opts::OpenIn {
    if present {
        opts::OpenIn::Editor
    } else {
        opts::OpenIn::Nothing
    }
}
