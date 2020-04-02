pub use cargo_mobile_macros::main;

use crate::opts;
use colored::Colorize as _;
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

pub fn bin_name(name: &str) -> String {
    format!("cargo {}", name)
}

#[derive(Clone, Copy, Debug, StructOpt)]
pub struct GlobalFlags {
    #[structopt(
        short = "v",
        long = "verbose",
        about = "Make life louder",
        global = true,
        multiple = true,
        parse(from_occurrences = opts::NoiseLevel::from_occurrences),
    )]
    pub noise_level: opts::NoiseLevel,
    #[structopt(
        long = "non-interactive",
        about = "Go with the flow",
        global = true,
        parse(from_flag = opts::Interactivity::from_flag),
    )]
    pub interactivity: opts::Interactivity,
}

#[derive(Clone, Copy, Debug, StructOpt)]
pub struct Clobbering {
    #[structopt(
        long = "force",
        about = "Clobber files with no remorse",
        parse(from_flag = opts::Clobbering::from_flag),
    )]
    pub clobbering: opts::Clobbering,
}

#[derive(Clone, Copy, Debug, StructOpt)]
pub struct Profile {
    #[structopt(
        long = "release",
        about = "Build with release optimizations",
        parse(from_flag = opts::Profile::from_flag),
    )]
    pub profile: opts::Profile,
}

pub type TextWrapper = textwrap::Wrapper<'static, textwrap::NoHyphenation>;

pub trait ExecError: Debug + Display {
    fn code(&self) -> i8 {
        1
    }

    fn color(&self) -> colored::Color {
        colored::Color::BrightRed
    }
}

impl ExecError for std::convert::Infallible {}
impl ExecError for std::io::Error {}

pub trait Exec: Debug + StructOpt {
    type Error: ExecError;

    fn global_flags(&self) -> GlobalFlags;

    fn exec(self, wrapper: &TextWrapper) -> Result<(), Self::Error>;
}

fn get_args(name: &str) -> Vec<String> {
    let mut args: Vec<String> = std::env::args().collect();
    // Running this as a cargo subcommand gives us our name as an argument,
    // so let's just discard that...
    if args.get(1).map(String::as_str) == Some(name) {
        args.remove(1);
    }
    args
}

fn init_logging(noise_level: opts::NoiseLevel) {
    use env_logger::{Builder, Env};
    let default_level = match noise_level {
        opts::NoiseLevel::Polite => "warn",
        opts::NoiseLevel::LoudAndProud => "cargo_mobile=info,bossy=info",
        opts::NoiseLevel::FranklyQuitePedantic => "info,bossy=debug",
    };
    let env = Env::default().default_filter_or(default_level);
    Builder::from_env(env).init();
}

#[derive(Debug)]
enum Exit {
    Display(String, i8, colored::Color),
    Clap(clap::Error),
}

impl Exit {
    fn display(err: impl ExecError) -> Self {
        Self::Display(format!("{}", err), err.code(), err.color())
    }

    fn do_the_thing(self, wrapper: Option<TextWrapper>) -> ! {
        match self {
            Self::Display(err, code, color) => {
                eprintln!(
                    "{}",
                    if let Some(wrapper) = wrapper {
                        wrapper.fill(&err).color(color)
                    } else {
                        err.color(color)
                    }
                );
                // We only expose access to the 8 lsb of the exit code, since:
                // https://doc.rust-lang.org/std/process/fn.exit.html#platform-specific-behavior
                std::process::exit(code as i32)
            }
            Self::Clap(err) => err.exit(),
        }
    }

    fn main(inner: impl FnOnce(&TextWrapper) -> Result<(), Self>) {
        let wrapper = TextWrapper::with_splitter(textwrap::termwidth(), textwrap::NoHyphenation);
        if let Err(exit) = inner(&wrapper) {
            exit.do_the_thing(Some(wrapper))
        }
    }
}

pub fn exec<E: Exec>(name: &str) {
    Exit::main(|wrapper| {
        let input = E::from_iter_safe(get_args(name)).map_err(Exit::Clap)?;
        init_logging(input.global_flags().noise_level);
        input.exec(wrapper).map_err(Exit::display)
    })
}
