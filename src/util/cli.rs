use crate::{opts, util};
use colored::Colorize as _;
use once_cell_regex::exports::once_cell::sync::Lazy;
use std::fmt::{Debug, Display};
use structopt::{
    clap::{self, AppSettings},
    StructOpt,
};

pub static GLOBAL_SETTINGS: &[AppSettings] = &[
    AppSettings::ColoredHelp,
    AppSettings::DeriveDisplayOrder,
    AppSettings::VersionlessSubcommands,
];

pub static SETTINGS: &[AppSettings] = &[AppSettings::SubcommandRequiredElseHelp];

pub fn bin_name(name: &str) -> String {
    format!("cargo {}", name)
}

pub static VERSION_SHORT: &str = concat!("v", env!("CARGO_PKG_VERSION"));

pub static VERSION_LONG: Lazy<String> = Lazy::new(|| match util::installed_commit_msg() {
    Ok(Some(msg)) => format!("{}\nContains commits up to {:?}", VERSION_SHORT, msg),
    Ok(None) => VERSION_SHORT.to_owned(),
    Err(err) => {
        log::error!("failed to get current commit msg: {}", err);
        VERSION_SHORT.to_owned()
    }
});

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
        parse(from_flag = opts::NonInteractive::from_bool),
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

    pub fn exit_code(&self) -> i8 {
        match self {
            Self::Victory => 0,
            _ => 1,
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

    pub fn exit_code(&self) -> i8 {
        self.label.exit_code()
    }

    fn format(&self, wrapper: &TextWrapper) -> String {
        static INDENT: &str = "    ";
        let head = if colored::control::SHOULD_COLORIZE.should_colorize() {
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
        format!("{}\n{}\n", head, wrapper.fill(&self.details))
    }

    pub fn print(&self, wrapper: &TextWrapper) {
        let s = self.format(wrapper);
        if matches!(self.label, Label::Error) {
            eprint!("{}", s)
        } else {
            print!("{}", s)
        }
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
        opts::NoiseLevel::LoudAndProud => {
            "cargo_mobile=info,cargo_android=info,cargo_apple=info,bossy=info,hit=info"
        }
        opts::NoiseLevel::FranklyQuitePedantic => {
            "info,cargo_mobile=debug,cargo_android=debug,cargo_apple=debug,bossy=debug,hit=debug"
        }
    };
    let env = Env::default().default_filter_or(default_level);
    Builder::from_env(env).init();
}

#[derive(Debug)]
pub enum Exit {
    Report(Report),
    Clap(clap::Error),
}

impl Exit {
    fn report(reportable: impl Reportable) -> Self {
        log::info!("exiting with {:#?}", reportable);
        Self::Report(reportable.report())
    }

    fn do_the_thing(self, wrapper: TextWrapper) -> ! {
        match self {
            Self::Report(report) => {
                report.print(&wrapper);
                std::process::exit(report.label.exit_code().into())
            }
            Self::Clap(err) => err.exit(),
        }
    }

    pub fn main(inner: impl FnOnce(&TextWrapper) -> Result<(), Self>) {
        let wrapper = TextWrapper::with_splitter(textwrap::termwidth(), textwrap::NoHyphenation);
        if let Err(exit) = inner(&wrapper) {
            exit.do_the_thing(wrapper)
        }
    }
}

pub fn exec<E: Exec>(name: &str) {
    Exit::main(|wrapper| {
        let args = get_args(name);
        let input = E::from_iter_safe(&args).map_err(Exit::Clap)?;
        init_logging(input.global_flags().noise_level);
        log::debug!("raw args: {:#?}", args);
        input.exec(wrapper).map_err(Exit::report)
    })
}
