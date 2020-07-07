use crate::opts;
use colored::Colorize as _;
use std::fmt::{Debug, Display};
use structopt::{
    clap::{self, AppSettings},
    StructOpt,
};

pub static GLOBAL_SETTINGS: &'static [AppSettings] = &[
    AppSettings::ColoredHelp,
    AppSettings::DeriveDisplayOrder,
    AppSettings::VersionlessSubcommands,
];

pub static SETTINGS: &'static [AppSettings] = &[AppSettings::SubcommandRequiredElseHelp];

pub fn bin_name(name: &str) -> String {
    format!("cargo {}", name)
}

#[derive(Clone, Copy, Debug, StructOpt)]
pub struct GlobalFlags {
    #[structopt(
        short = "v",
        long = "verbose",
        help = "Vomit out extensive logging",
        global = true,
        multiple = true,
        parse(from_occurrences = opts::NoiseLevel::from_occurrences),
    )]
    pub noise_level: opts::NoiseLevel,
    #[structopt(
        short = "y",
        long = "non-interactive",
        help = "Never prompt for input",
        global = true,
        parse(from_flag = opts::NonInteractive::from_flag),
    )]
    pub non_interactive: opts::NonInteractive,
}

#[derive(Clone, Copy, Debug, StructOpt)]
pub struct SkipDevTools {
    #[structopt(
        long = "skip-dev-tools",
        help = "Skip optional tools that help when writing code",
        parse(from_flag = opts::SkipDevTools::from_bool),
    )]
    pub skip_dev_tools: opts::SkipDevTools,
}

#[derive(Clone, Copy, Debug, StructOpt)]
pub struct ReinstallDeps {
    #[structopt(
        long = "reinstall-deps",
        help = "Reinstall dependencies",
        parse(from_flag = opts::ReinstallDeps::from_bool),
    )]
    pub reinstall_deps: opts::ReinstallDeps,
}

#[derive(Clone, Copy, Debug, StructOpt)]
pub struct Profile {
    #[structopt(
        long = "release",
        help = "Build with release optimizations",
        parse(from_flag = opts::Profile::from_flag),
    )]
    pub profile: opts::Profile,
}

pub type TextWrapper = textwrap::Wrapper<'static, textwrap::NoHyphenation>;

#[derive(Clone, Copy, Debug)]
pub enum Label {
    Error,
    ActionRequest,
    Victory,
}

impl Label {
    pub fn color(&self) -> colored::Color {
        match self {
            Self::Error => colored::Color::BrightRed,
            Self::ActionRequest => colored::Color::BrightMagenta,
            Self::Victory => colored::Color::BrightGreen,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::ActionRequest => "action request",
            Self::Victory => "victory",
        }
    }
}

#[derive(Debug)]
pub struct Report {
    label: Label,
    msg: String,
    details: String,
}

impl Report {
    pub fn new(label: Label, msg: impl Display, details: impl Display) -> Self {
        Self {
            label,
            msg: format!("{}", msg),
            details: format!("{}", details),
        }
    }

    pub fn error(msg: impl Display, details: impl Display) -> Self {
        Self::new(Label::Error, msg, details)
    }

    pub fn action_request(msg: impl Display, details: impl Display) -> Self {
        Self::new(Label::ActionRequest, msg, details)
    }

    pub fn victory(msg: impl Display, details: impl Display) -> Self {
        Self::new(Label::Victory, msg, details)
    }

    pub fn render(&self, wrapper: &TextWrapper, non_interactive: opts::NonInteractive) -> String {
        static INDENT: &'static str = "    ";
        let head = if non_interactive.no() && colored::control::SHOULD_COLORIZE.should_colorize() {
            wrapper.fill(&format!(
                "{} {}",
                format!("{}:", self.label.as_str())
                    .color(self.label.color())
                    .bold(),
                self.msg.color(self.label.color())
            ))
        } else {
            wrapper.fill(&format!("{}: {}", self.label.as_str(), &self.msg))
        };
        let wrapper = wrapper
            .clone()
            .initial_indent(INDENT)
            .subsequent_indent(INDENT);
        format!("{}\n{}", head, wrapper.fill(&self.details))
    }
}

pub trait Reportable: Debug {
    fn report(&self) -> Report;
}

pub trait Exec: Debug + StructOpt {
    type Report: Reportable;

    fn global_flags(&self) -> GlobalFlags;

    fn exec(self, wrapper: &TextWrapper) -> Result<(), Self::Report>;
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
        opts::NoiseLevel::FranklyQuitePedantic => "info,cargo_mobile=debug,bossy=debug",
    };
    let env = Env::default().default_filter_or(default_level);
    Builder::from_env(env).init();
}

#[derive(Debug)]
enum Exit {
    Report(Report, opts::NonInteractive),
    Clap(clap::Error),
}

impl Exit {
    fn report(reportable: impl Reportable, non_interactive: opts::NonInteractive) -> Self {
        log::info!("exiting with {:#?}", reportable);
        Self::Report(reportable.report(), non_interactive)
    }

    fn do_the_thing(self, wrapper: TextWrapper) -> ! {
        match self {
            Self::Report(report, non_interactive) => {
                eprintln!("{}", report.render(&wrapper, non_interactive));
                std::process::exit(1)
            }
            Self::Clap(err) => err.exit(),
        }
    }

    fn main(inner: impl FnOnce(&TextWrapper) -> Result<(), Self>) {
        let wrapper = TextWrapper::with_splitter(textwrap::termwidth(), textwrap::NoHyphenation);
        if let Err(exit) = inner(&wrapper) {
            exit.do_the_thing(wrapper)
        }
    }
}

pub fn exec<E: Exec>(name: &str) {
    Exit::main(|wrapper| {
        let input = E::from_iter_safe(get_args(name)).map_err(Exit::Clap)?;
        let GlobalFlags {
            noise_level,
            non_interactive,
        } = input.global_flags();
        init_logging(noise_level);
        input
            .exec(wrapper)
            .map_err(|report| Exit::report(report, non_interactive))
    })
}
