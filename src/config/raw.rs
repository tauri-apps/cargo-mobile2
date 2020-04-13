use super::{app, TemplatePack};
use crate::{
    android, apple,
    util::{cli::TextWrapper, submodule::Submodule},
};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum PromptError {
    AppFailed(app::PromptError),
    AppleFailed(apple::config::PromptError),
}

impl Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppFailed(err) => {
                write!(f, "Failed to prompt for `{}` config: {}", app::KEY, err)
            }
            Self::AppleFailed(err) => {
                write!(f, "Failed to prompt for `{}` config: {}", apple::NAME, err)
            }
        }
    }
}

#[derive(Debug)]
pub enum DetectError {
    AppFailed(app::DetectError),
    AppleFailed(apple::config::DetectError),
}

impl Display for DetectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppFailed(err) => write!(f, "Failed to detect `{}` config: {}", app::KEY, err),
            Self::AppleFailed(err) => {
                write!(f, "Failed to detect `{}` config: {}", apple::NAME, err)
            }
        }
    }
}

fn default_template_packs_and_submodules() -> (Option<Vec<TemplatePack>>, Option<Vec<Submodule>>) {
    let brainium = false;
    if brainium {
        (
            Some(vec![TemplatePack::with_src(
                "~/.cargo-mobile/templates/rust-lib-app",
            )]),
            Some(vec![Submodule::with_remote_and_path(
                "git@bitbucket.org:brainium/rust_lib.git",
                "rust-lib",
            )]),
        )
    } else {
        (
            Some(vec![TemplatePack::with_src(
                "~/.cargo-mobile/templates/wgpu-app",
            )]),
            None,
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    pub app: app::Raw,
    pub template_packs: Option<Vec<TemplatePack>>,
    pub submodules: Option<Vec<Submodule>>,
    pub android: Option<android::config::Raw>,
    pub apple: Option<apple::config::Raw>,
}

impl Raw {
    pub fn prompt(wrapper: &TextWrapper) -> Result<Self, PromptError> {
        let app = app::Raw::prompt(wrapper).map_err(PromptError::AppFailed)?;
        let (template_packs, submodules) = default_template_packs_and_submodules();
        let apple = apple::config::Raw::prompt(wrapper).map_err(PromptError::AppleFailed)?;
        Ok(Self {
            app,
            template_packs,
            submodules,
            android: None,
            apple: Some(apple),
        })
    }

    pub fn detect() -> Result<Self, DetectError> {
        let app = app::Raw::detect().map_err(DetectError::AppFailed)?;
        let (template_packs, submodules) = default_template_packs_and_submodules();
        let apple = apple::config::Raw::detect().map_err(DetectError::AppleFailed)?;
        Ok(Self {
            app,
            template_packs,
            submodules,
            android: None,
            apple: Some(apple),
        })
    }
}
