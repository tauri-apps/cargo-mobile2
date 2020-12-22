use super::{common_email_providers::COMMON_EMAIL_PROVIDERS, name};
use crate::{
    templating,
    util::{cli::TextWrapper, prompt, Git},
};
use colored::{Color, Colorize as _};
use heck::{KebabCase as _, TitleCase as _};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::{self, Display},
    io,
    path::PathBuf,
};

#[derive(Debug)]
enum DefaultDomainError {
    FailedToGetGitEmailAddr(bossy::Error),
    FailedToParseEmailAddr,
}

fn default_domain() -> Result<Option<String>, DefaultDomainError> {
    let email = Git::new(".".as_ref())
        .user_email()
        .map_err(DefaultDomainError::FailedToGetGitEmailAddr)?;
    let domain = email
        .trim()
        .split('@')
        .last()
        .ok_or(DefaultDomainError::FailedToParseEmailAddr)?;
    Ok(
        if !COMMON_EMAIL_PROVIDERS.contains(&domain)
            && publicsuffix::Domain::has_valid_syntax(&domain)
        {
            Some(domain.to_owned())
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
    domain: String,
}

impl Defaults {
    fn new() -> Result<Self, DefaultsError> {
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
            domain: default_domain()
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
    DomainPromptFailed(io::Error),
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
            Self::DomainPromptFailed(err) => write!(f, "Failed to prompt for domain: {}", err),
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
    pub stylized_name: Option<String>,
    pub domain: String,
    pub asset_dir: Option<String>,
    #[cfg(feature = "brainium")]
    pub template_pack: Option<String>,
    #[cfg(not(feature = "brainium"))]
    pub template_pack: String,
}

impl Raw {
    pub fn detect() -> Result<Self, DetectError> {
        let defaults = Defaults::new().map_err(DetectError::DefaultsFailed)?;
        Ok(Self {
            name: defaults.name.ok_or_else(|| DetectError::NameNotDetected)?,
            stylized_name: Some(defaults.stylized_name),
            domain: defaults.domain,
            asset_dir: None,
            #[cfg(feature = "brainium")]
            template_pack: None,
            #[cfg(not(feature = "brainium"))]
            template_pack: super::DEFAULT_TEMPLATE_PACK.to_owned(),
        })
    }

    pub fn prompt(wrapper: &TextWrapper) -> Result<Self, PromptError> {
        let defaults = Defaults::new().map_err(PromptError::DefaultsFailed)?;
        let (name, default_stylized) = Self::prompt_name(wrapper, &defaults)?;
        let stylized_name = Self::prompt_stylized_name(&name, default_stylized)?;
        let domain = Self::prompt_domain(wrapper, &defaults)?;
        let template_pack = Self::prompt_template_pack(wrapper)?;
        #[cfg(feature = "brainium")]
        let template_pack = Some(template_pack).filter(|pack| pack != super::DEFAULT_TEMPLATE_PACK);
        Ok(Self {
            name,
            stylized_name: Some(stylized_name),
            domain,
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
            let response = prompt::default(
                "Project name",
                default_name.as_ref().map(|s| s.as_str()),
                None,
            )
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
        let stylized = default_stylized
            .unwrap_or_else(|| name.replace("-", " ").replace("_", " ").to_title_case());
        prompt::default("Stylized name", Some(&stylized), None)
            .map_err(PromptError::StylizedNamePromptFailed)
    }

    fn prompt_domain(wrapper: &TextWrapper, defaults: &Defaults) -> Result<String, PromptError> {
        Ok(loop {
            let response = prompt::default("Domain", Some(&defaults.domain), None)
                .map_err(PromptError::DomainPromptFailed)?;
            if publicsuffix::Domain::has_valid_syntax(&response) {
                break response;
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
                .map(|pack| pack.clone());
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
