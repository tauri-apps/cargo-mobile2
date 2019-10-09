use super::default::{DefaultShared, DetectError};
use crate::{
    config::{app_name, DefaultConfigTrait as _, RequiredConfigTrait},
    util::{self, prompt},
};
use colored::*;
use heck::TitleCase as _;
use serde::Serialize;
use std::{
    fmt::{self, Display},
    io,
};

#[derive(Debug)]
pub enum PromptError {
    DefaultConfigDetectionFailed(DetectError),
    AppNamePromptFailed(io::Error),
    StylizedAppNamePromptFailed(io::Error),
    DomainPromptFailed(io::Error),
}

impl Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefaultConfigDetectionFailed(err) => {
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

#[derive(Debug, Serialize)]
pub struct RequiredShared {
    pub app_name: String,
    pub stylized_app_name: String,
    pub domain: String,
}

impl RequiredConfigTrait for RequiredShared {
    type PromptError = PromptError;
    fn prompt(wrapper: &util::TextWrapper) -> Result<Self, Self::PromptError> {
        let defaults =
            DefaultShared::detect().map_err(PromptError::DefaultConfigDetectionFailed)?;
        let (app_name, default_stylized) = Self::prompt_app_name(wrapper, &defaults)?;
        let stylized_app_name = Self::prompt_stylized_app_name(&app_name, default_stylized)?;
        let domain = Self::prompt_domain(wrapper, &defaults)?;
        Ok(Self {
            app_name,
            stylized_app_name,
            domain,
        })
    }
}

impl RequiredShared {
    fn prompt_app_name(
        wrapper: &util::TextWrapper,
        defaults: &DefaultShared,
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
        defaults: &DefaultShared,
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
