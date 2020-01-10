pub use ginit_macros::main;

use crate::{
    config::{umbrella::Umbrella, ConfigTrait},
    opts,
    util::{self, init_text_wrapper, TextWrapper},
};
use colored::*;
use std::fmt::{Debug, Display};
use structopt::{
    clap::{self, AppSettings},
    StructOpt,
};

pub static SETTINGS: &'static [AppSettings] = &[
    AppSettings::ColoredHelp,
    AppSettings::DeriveDisplayOrder,
    AppSettings::SubcommandRequiredElseHelp,
    AppSettings::VersionlessSubcommands,
];

#[derive(Clone, Copy, Debug, StructOpt)]
pub struct GlobalFlags {
    #[structopt(
        short = "v",
        long = "verbose",
        about = "Make life louder",
        global = true,
        multiple = true,
        parse(from_occurrences = noise_level_from_occurrences),
    )]
    pub noise_level: opts::NoiseLevel,
    #[structopt(
        long = "non-interactive",
        about = "Go with the flow",
        global = true,
        parse(from_flag = interactivity_from_presence),
    )]
    pub interactivity: opts::Interactivity,
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

#[derive(Clone, Copy, Debug, StructOpt)]
pub enum Barebones {
    #[structopt(name = "config-gen", about = "Generate configuration", setting = AppSettings::Hidden)]
    ConfigGen,
    #[structopt(
        name = "init",
        about = "Creates a new project in the current working directory"
    )]
    Init {
        #[structopt(flatten)]
        clobbering: Clobbering,
    },
}

pub fn noise_level_from_occurrences(occurrences: u64) -> opts::NoiseLevel {
    match occurrences {
        0 => opts::NoiseLevel::Polite,
        1 => opts::NoiseLevel::LoudAndProud,
        _ => opts::NoiseLevel::FranklyQuitePedantic,
    }
}

pub fn interactivity_from_presence(present: bool) -> opts::Interactivity {
    if present {
        opts::Interactivity::None
    } else {
        opts::Interactivity::Full
    }
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

pub fn get_args(name: impl AsRef<str>) -> Vec<String> {
    let mut args: Vec<String> = std::env::args().collect();
    // Running this as a cargo subcommand gives us our name as an argument,
    // so let's just discard that...
    if args.get(1).map(String::as_str) == Some(name.as_ref()) {
        args.remove(1);
    }
    args
}

pub trait Exec: Debug + StructOpt {
    type Config: ConfigTrait;
    type Error: Debug + Display;

    fn global_flags(&self) -> GlobalFlags;

    fn exec(self, config: Option<Self::Config>, wrapper: &TextWrapper) -> Result<(), Self::Error>;
}

pub fn init_logging(noise_level: opts::NoiseLevel) {
    use env_logger::{Builder, Env};
    let default_level = match noise_level {
        opts::NoiseLevel::Polite => "warn",
        opts::NoiseLevel::LoudAndProud => "ginit=info",
        opts::NoiseLevel::FranklyQuitePedantic => "info",
    };
    let env = Env::default().default_filter_or(default_level);
    Builder::from_env(env).init();
}

#[derive(Debug)]
pub enum NonZeroExit {
    Display(String),
    Clap(clap::Error),
}

impl NonZeroExit {
    pub fn display(err: impl Display) -> Self {
        Self::Display(util::display(err))
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

    pub fn exec<E: Exec>(name: &str) {
        Self::main(|wrapper| {
            let input = E::from_iter_safe(get_args(name)).map_err(Self::Clap)?;
            init_logging(input.global_flags().noise_level);
            let config = Umbrella::load_plugin(name).map_err(Self::display)?;
            input.exec(config, wrapper).map_err(Self::display)
        })
    }
}
