use crate::{
    android, apple,
    config::{self, Config},
    opts, project,
    steps::{self, Steps},
    util,
};
use std::fmt::{self, Display};
use structexec::{Interactivity, TextWrapper};

pub static STEPS: &'static [&'static str] = &["project", "android", "apple"];

#[derive(Debug)]
pub enum Error {
    ConfigLoadOrGenFailed(config::LoadOrGenError),
    OnlyParseFailed(steps::NotRegistered),
    SkipParseFailed(steps::NotRegistered),
    StepNotRegistered(steps::NotRegistered),
    ProjectInitFailed(project::Error),
    AndroidEnvFailed(android::env::Error),
    AndroidInitFailed(android::project::Error),
    AppleInitFailed(apple::project::Error),
    OpenInEditorFailed(util::OpenInEditorError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigLoadOrGenFailed(err) => write!(f, "{}", err),
            Self::OnlyParseFailed(err) => write!(f, "Failed to parse `only` step list: {}", err),
            Self::SkipParseFailed(err) => write!(f, "Failed to parse `skip` step list: {}", err),
            Self::StepNotRegistered(err) => write!(f, "{}", err),
            Self::ProjectInitFailed(err) => write!(f, "Failed to generate base project: {}", err),
            Self::AndroidEnvFailed(err) => write!(f, "Failed to detect Android env: {}", err),
            Self::AndroidInitFailed(err) => write!(f, "Failed to init Android project: {}", err),
            Self::AppleInitFailed(err) => write!(f, "Failed to init Apple project: {}", err),
            Self::OpenInEditorFailed(err) => write!(f, "Failed to open project in editor (your project generated successfully though, so no worries!): {}", err),
        }
    }
}

pub fn exec(
    interactivity: Interactivity,
    clobbering: opts::Clobbering,
    open_in: opts::OpenIn,
    only: Option<Vec<String>>,
    skip: Option<Vec<String>>,
    wrapper: &TextWrapper,
) -> Result<Config, Error> {
    let config =
        Config::load_or_gen(".", interactivity, wrapper).map_err(Error::ConfigLoadOrGenFailed)?;
    let bike = config.build_a_bike();
    let step_registry = steps::Registry::new(STEPS);
    let steps = {
        let only = only
            .as_ref()
            .map(|only| Steps::parse(&step_registry, only.as_slice()))
            .unwrap_or_else(|| Ok(Steps::new_all_set(&step_registry)))
            .map_err(Error::OnlyParseFailed)?;
        let skip = skip
            .as_ref()
            .map(|skip| Steps::parse(&step_registry, skip.as_slice()))
            .unwrap_or_else(|| Ok(Steps::new_all_unset(&step_registry)))
            .map_err(Error::SkipParseFailed)?;
        Steps::from_bits(&step_registry, only.bits() & !skip.bits())
    };
    if steps.is_set("project") {
        project::gen(&config, &bike, clobbering).map_err(Error::ProjectInitFailed)?;
    }
    if steps.is_set("android") {
        let env = android::env::Env::new().map_err(Error::AndroidEnvFailed)?;
        android::project::gen(config.android(), &env, &bike, clobbering)
            .map_err(Error::AndroidInitFailed)?;
    }
    if steps.is_set("apple") {
        apple::project::gen(config.apple(), &bike, clobbering).map_err(Error::AppleInitFailed)?;
    }
    if open_in.editor() {
        util::open_in_editor(".").map_err(Error::OpenInEditorFailed)?;
    }
    Ok(config)
}
