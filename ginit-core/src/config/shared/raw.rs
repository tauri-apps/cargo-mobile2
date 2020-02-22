use super::detect::{DetectError, Detected};
use crate::{
    config::{app_name, RawConfigTrait},
    util::{self, prompt},
};
use colored::*;
use heck::TitleCase as _;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    io,
};

#[derive(Debug)]
pub enum UpgradeError {
    AppNameNotDetected,
}

impl Display for UpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppNameNotDetected => write!(f, "No app name was detected."),
        }
    }
}

#[derive(Debug)]
pub enum PromptError {
    DetectedConfigDetectionFailed(DetectError),
    AppNamePromptFailed(io::Error),
    StylizedAppNamePromptFailed(io::Error),
    DomainPromptFailed(io::Error),
}

impl Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DetectedConfigDetectionFailed(err) => {
                write!(f, "Failed to detect default config values: {}", err)
            }
            Self::AppNamePromptFailed(err) => write!(f, "Failed to prompt for app name: {}", err),
            Self::StylizedAppNamePromptFailed(err) => {
                write!(f, "Failed to prompt for stylized app name: {}", err)
            }
            Self::DomainPromptFailed(err) => write!(f, "Failed to prompt for domain: {}", err),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Raw {
    #[serde(alias = "app-name")]
    pub app_name: String,
    #[serde(alias = "stylized-app-name")]
    pub stylized_app_name: Option<String>,
    pub domain: String,
    #[serde(alias = "app-root")]
    pub app_root: Option<String>,
    pub plugins: Option<Vec<String>>,
}

impl RawConfigTrait for Raw {
    type Detected = Detected;

    type FromDetectedError = UpgradeError;
    fn from_detected(detected: Self::Detected) -> Result<Self, Self::FromDetectedError> {
        Ok(Self {
            app_name: detected
                .app_name
                .ok_or_else(|| UpgradeError::AppNameNotDetected)?,
            stylized_app_name: Some(detected.stylized_app_name),
            domain: detected.domain,
            app_root: None,
            plugins: Some(detected.plugins),
        })
    }

    type FromPromptError = PromptError;
    fn from_prompt(
        detected: Self::Detected,
        wrapper: &util::TextWrapper,
    ) -> Result<Self, Self::FromPromptError> {
        let (app_name, default_stylized) = Self::prompt_app_name(wrapper, &detected)?;
        let stylized_app_name = Self::prompt_stylized_app_name(&app_name, default_stylized)?;
        let domain = Self::prompt_domain(wrapper, &detected)?;
        Ok(Self {
            app_name,
            stylized_app_name: Some(stylized_app_name),
            domain,
            app_root: None,
            plugins: Some(detected.plugins),
        })
    }
}

impl Raw {
    fn prompt_app_name(
        wrapper: &util::TextWrapper,
        defaults: &Detected,
    ) -> Result<(String, Option<String>), PromptError> {
        let mut default_app_name = defaults.app_name.clone();
        let mut app_name = None;
        let mut rejected = None;
        let mut default_stylized = None;
        while let None = app_name {
            let response = prompt::default(
                "App name",
                default_app_name.as_ref().map(|s| s.as_str()),
                None,
            )
            .map_err(PromptError::AppNamePromptFailed)?;
            match app_name::validate(response.clone()) {
                Ok(response) => {
                    if default_app_name == Some(response.clone()) {
                        if rejected.is_some() {
                            default_stylized = rejected.take();
                        } else {
                            default_stylized = Some(defaults.stylized_app_name.clone());
                        }
                    }
                    app_name = Some(response);
                }
                Err(err) => {
                    rejected = Some(response);
                    println!(
                        "{}",
                        wrapper
                            .fill(&format!("Gosh, that's not a valid app name! {}", err))
                            .bright_magenta()
                    );
                    if let Some(suggested) = err.suggested() {
                        default_app_name = Some(suggested.to_owned());
                    }
                }
            }
        }
        Ok((app_name.unwrap(), default_stylized))
    }

    fn prompt_stylized_app_name(
        app_name: &str,
        default_stylized: Option<String>,
    ) -> Result<String, PromptError> {
        let stylized = default_stylized
            .unwrap_or_else(|| app_name.replace("-", " ").replace("_", " ").to_title_case());
        prompt::default("Stylized app name", Some(&stylized), None)
            .map_err(PromptError::StylizedAppNamePromptFailed)
    }

    fn prompt_domain(
        wrapper: &util::TextWrapper,
        defaults: &Detected,
    ) -> Result<String, PromptError> {
        let mut domain = None;
        while let None = domain {
            let response = prompt::default("Domain", Some(&defaults.domain), None)
                .map_err(PromptError::DomainPromptFailed)?;
            if publicsuffix::Domain::has_valid_syntax(&response) {
                domain = Some(response);
            } else {
                println!(
                    "{}",
                    wrapper
                        .fill(&format!(
                            "Sorry, but {:?} isn't valid domain syntax.",
                            response
                        ))
                        .bright_magenta()
                );
            }
        }
        Ok(domain.unwrap())
    }
}
