use super::{common_email_providers::COMMON_EMAIL_PROVIDERS, identifier, name};
use crate::{
    templating,
    util::{cli::TextWrapper, prompt, Git},
};
use colored::{Color, Colorize as _};
use heck::{ToKebabCase as _, ToTitleCase as _};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::{self, Display},
    io,
    path::PathBuf,
};

#[derive(Debug)]
enum DefaultIdentifierError {
    FailedToGetGitEmailAddr(#[allow(unused)] std::io::Error),
    FailedToParseEmailAddr,
}

fn default_identifier(_wrapper: &TextWrapper) -> Result<Option<String>, DefaultIdentifierError> {
    let email = Git::new(".".as_ref())
        .user_email()
        .map_err(DefaultIdentifierError::FailedToGetGitEmailAddr)?;
    let identifier = email
        .trim()
        .split('@')
        .last()
        .ok_or(DefaultIdentifierError::FailedToParseEmailAddr)?;
    Ok(
        if !COMMON_EMAIL_PROVIDERS.contains(&identifier)
            && identifier::check_identifier_syntax(identifier).is_ok()
        {
            #[cfg(not(feature = "brainium"))]
            if identifier == "brainiumstudios.com" {
                crate::util::cli::Report::action_request(
                    "You have a Brainium email address, but you're using a non-Brainium installation of cargo-mobile2!",
                    "If that's not intentional, run `cargo install --force --git https://github.com/tauri-apps/cargo-mobile2 --features brainium`",
                ).print(_wrapper);
            }
            Some(identifier.to_owned())
        } else {
            None
        },
    )
}

#[derive(Debug)]
pub enum DefaultsError {
    CurrentDirFailed(io::Error),
    CurrentDirHasNoName(PathBuf),
    CurrentDirInvalidUtf8(PathBuf),
}

impl Display for DefaultsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentDirFailed(err) => {
                write!(f, "Failed to get current working directory: {}", err)
            }
            Self::CurrentDirHasNoName(cwd) => {
                write!(f, "Current working directory has no name: {:?}", cwd)
            }
            Self::CurrentDirInvalidUtf8(cwd) => write!(
                f,
                "Current working directory contained invalid UTF-8: {:?}",
                cwd
            ),
        }
    }
}

#[derive(Debug)]
struct Defaults {
    name: Option<String>,
    stylized_name: String,
    identifier: String,
}

impl Defaults {
    fn new(wrapper: &TextWrapper) -> Result<Self, DefaultsError> {
        let cwd = env::current_dir().map_err(DefaultsError::CurrentDirFailed)?;
        let dir_name = cwd
            .file_name()
            .ok_or_else(|| DefaultsError::CurrentDirHasNoName(cwd.clone()))?;
        let dir_name = dir_name
            .to_str()
            .ok_or_else(|| DefaultsError::CurrentDirInvalidUtf8(cwd.clone()))?;
        Ok(Self {
            name: name::transliterate(&dir_name.to_kebab_case()),
            stylized_name: dir_name.to_title_case(),
            identifier: default_identifier(wrapper)
                .ok()
                .flatten()
                .unwrap_or_else(|| "example.com".to_owned()),
        })
    }
}

#[derive(Debug)]
pub enum DetectError {
    DefaultsFailed(DefaultsError),
    NameNotDetected,
}

impl Display for DetectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefaultsFailed(err) => write!(f, "Failed to detect default values: {}", err),
            Self::NameNotDetected => write!(f, "No app name was detected."),
        }
    }
}

#[derive(Debug)]
pub enum PromptError {
    DefaultsFailed(DefaultsError),
    NamePromptFailed(io::Error),
    StylizedNamePromptFailed(io::Error),
    IdentifierPromptFailed(io::Error),
    ListTemplatePacksFailed(templating::ListError),
    TemplatePackPromptFailed(io::Error),
}

impl Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefaultsFailed(err) => write!(f, "Failed to detect default values: {}", err),
            Self::NamePromptFailed(err) => write!(f, "Failed to prompt for name: {}", err),
            Self::StylizedNamePromptFailed(err) => {
                write!(f, "Failed to prompt for stylized name: {}", err)
            }
            Self::IdentifierPromptFailed(err) => {
                write!(f, "Failed to prompt for identifier: {}", err)
            }
            Self::ListTemplatePacksFailed(err) => write!(f, "{}", err),
            Self::TemplatePackPromptFailed(err) => {
                write!(f, "Failed to prompt for template pack: {}", err)
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    pub name: String,
    pub lib_name: Option<String>,
    pub stylized_name: Option<String>,
    pub identifier: String,
    pub asset_dir: Option<String>,
    pub template_pack: Option<String>,
}

impl Raw {
    pub fn detect(wrapper: &TextWrapper) -> Result<Self, DetectError> {
        let defaults = Defaults::new(wrapper).map_err(DetectError::DefaultsFailed)?;
        Ok(Self {
            name: defaults.name.ok_or_else(|| DetectError::NameNotDetected)?,
            lib_name: None,
            stylized_name: Some(defaults.stylized_name),
            identifier: defaults.identifier,
            asset_dir: None,
            template_pack: Some(super::DEFAULT_TEMPLATE_PACK.to_owned())
                .filter(|pack| pack != super::IMPLIED_TEMPLATE_PACK),
        })
    }

    pub fn prompt(wrapper: &TextWrapper) -> Result<Self, PromptError> {
        let defaults = Defaults::new(wrapper).map_err(PromptError::DefaultsFailed)?;
        let (name, default_stylized) = Self::prompt_name(wrapper, &defaults)?;
        let stylized_name = Self::prompt_stylized_name(&name, default_stylized)?;
        let identifier = Self::prompt_identifier(wrapper, &defaults)?;
        let template_pack = Some(Self::prompt_template_pack(wrapper)?)
            .filter(|pack| pack != super::IMPLIED_TEMPLATE_PACK);
        Ok(Self {
            name,
            lib_name: None,
            stylized_name: Some(stylized_name),
            identifier,
            asset_dir: None,
            template_pack,
        })
    }
}

impl Raw {
    fn prompt_name(
        wrapper: &TextWrapper,
        defaults: &Defaults,
    ) -> Result<(String, Option<String>), PromptError> {
        let mut default_name = defaults.name.clone();
        let mut rejected = None;
        let mut default_stylized = None;
        let name = loop {
            let response = prompt::default("Project name", default_name.as_deref(), None)
                .map_err(PromptError::NamePromptFailed)?;
            match name::validate(response.clone()) {
                Ok(response) => {
                    if default_name == Some(response.clone()) {
                        if rejected.is_some() {
                            default_stylized = rejected.take();
                        } else {
                            default_stylized = Some(defaults.stylized_name.clone());
                        }
                    }
                    break response;
                }
                Err(err) => {
                    rejected = Some(response);
                    println!(
                        "{}",
                        wrapper
                            .fill(&format!("Gosh, that's not a valid project name! {}", err))
                            .bright_magenta()
                    );
                    if let Some(suggested) = err.suggested() {
                        default_name = Some(suggested.to_owned());
                    }
                }
            }
        };
        Ok((name, default_stylized))
    }

    fn prompt_stylized_name(
        name: &str,
        default_stylized: Option<String>,
    ) -> Result<String, PromptError> {
        let stylized =
            default_stylized.unwrap_or_else(|| name.replace(['-', '_'], " ").to_title_case());
        prompt::default("Stylized name", Some(&stylized), None)
            .map_err(PromptError::StylizedNamePromptFailed)
    }

    fn prompt_identifier(
        wrapper: &TextWrapper,
        defaults: &Defaults,
    ) -> Result<String, PromptError> {
        Ok(loop {
            let response = prompt::default("Identifier", Some(&defaults.identifier), None)
                .map_err(PromptError::IdentifierPromptFailed)?;
            match identifier::check_identifier_syntax(response.as_str()) {
                Ok(_) => break response,
                Err(err) => {
                    println!(
                        "{}",
                        wrapper.fill(&format!("Sorry! {}", err)).bright_magenta()
                    )
                }
            }
        })
    }

    pub fn prompt_template_pack(wrapper: &TextWrapper) -> Result<String, PromptError> {
        let packs = templating::list_app_packs().map_err(PromptError::ListTemplatePacksFailed)?;
        let mut default_pack = None;
        println!("Detected template packs:");
        for (index, pack) in packs.iter().enumerate() {
            let default = pack == super::DEFAULT_TEMPLATE_PACK;
            if default {
                default_pack = Some(index.to_string());
                println!(
                    "{}",
                    format!("  [{}] {}", index.to_string().bright_green(), pack,)
                        .bright_white()
                        .bold()
                );
            } else {
                println!("  [{}] {}", index.to_string().green(), pack);
            }
        }
        if packs.is_empty() {
            println!("  -- none --");
        }
        loop {
            println!("  Enter an {} for a template pack above.", "index".green(),);
            let pack_input = prompt::default(
                "Template pack",
                default_pack.as_deref(),
                Some(Color::BrightGreen),
            )
            .map_err(PromptError::TemplatePackPromptFailed)?;
            let pack_name = pack_input
                .parse::<usize>()
                .ok()
                .and_then(|index| packs.get(index))
                .cloned();
            if let Some(pack_name) = pack_name {
                break Ok(pack_name);
            } else {
                println!(
                    "{}",
                    wrapper
                        .fill("Uh-oh, you need to specify a template pack.")
                        .bright_magenta()
                );
            }
        }
    }
}
