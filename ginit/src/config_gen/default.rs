use super::{domain_blacklist::DOMAIN_BLACKLIST, RequiredConfig};
use crate::{config::app_name, ios};
use heck::{KebabCase as _, TitleCase as _};
use into_result::{command::CommandError, IntoResult as _};
use std::{env, fmt, io, path::PathBuf, process::Command};

#[derive(Debug)]
enum DefaultDomainError {
    FailedToGetGitEmailAddr(CommandError),
    EmailAddrInvalidUtf8(std::str::Utf8Error),
    FailedToParseEmailAddr,
}

fn default_domain() -> Result<Option<String>, DefaultDomainError> {
    let bytes = Command::new("git")
        .args(&["config", "user.email"])
        .output()
        .into_result()
        .map_err(DefaultDomainError::FailedToGetGitEmailAddr)
        .map(|output| output.stdout)?;
    let email = std::str::from_utf8(&bytes).map_err(DefaultDomainError::EmailAddrInvalidUtf8)?;
    let domain = email
        .trim()
        .split('@')
        .last()
        .ok_or(DefaultDomainError::FailedToParseEmailAddr)?;
    Ok(
        if !DOMAIN_BLACKLIST.contains(&domain) && publicsuffix::Domain::has_valid_syntax(&domain) {
            Some(domain.to_owned())
        } else {
            None
        },
    )
}

#[derive(Debug)]
pub enum DetectionError {
    CurrentDirFailed(io::Error),
    CurrentDirHasNoName(PathBuf),
    CurrentDirInvalidUtf8(PathBuf),
}

impl fmt::Display for DetectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DetectionError::CurrentDirFailed(err) => {
                write!(f, "Failed to get current working directory: {}", err)
            }
            DetectionError::CurrentDirHasNoName(cwd) => {
                write!(f, "Current working directory has no name: {:?}", cwd)
            }
            DetectionError::CurrentDirInvalidUtf8(cwd) => write!(
                f,
                "Current working directory contained invalid UTF-8: {:?}",
                cwd
            ),
        }
    }
}

#[derive(Debug)]
pub enum UpgradeError {
    DeveloperTeamLookupFailed(ios::teams::Error),
    AppNameNotDetected,
    DeveloperTeamsEmpty,
}

impl fmt::Display for UpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpgradeError::DeveloperTeamLookupFailed(err) => {
                write!(f, "Failed to find Apple developer teams: {}", err)
            }
            UpgradeError::AppNameNotDetected => write!(f, "No app name was detected."),
            UpgradeError::DeveloperTeamsEmpty => {
                write!(f, "No Apple developer teams were detected.")
            }
        }
    }
}

#[derive(Debug)]
pub struct DefaultConfig {
    pub app_name: Option<String>,
    pub stylized_app_name: String,
    pub domain: String,
}

impl DefaultConfig {
    pub fn detect() -> Result<Self, DetectionError> {
        let cwd = env::current_dir().map_err(DetectionError::CurrentDirFailed)?;
        let dir_name = cwd
            .file_name()
            .ok_or_else(|| DetectionError::CurrentDirHasNoName(cwd.clone()))?;
        let dir_name_str = dir_name
            .to_str()
            .ok_or_else(|| DetectionError::CurrentDirInvalidUtf8(cwd.clone()))?;
        let app_name = app_name::transliterate(&dir_name_str.to_kebab_case());
        let stylized_app_name = dir_name_str.to_title_case();
        let domain = default_domain()
            .ok()
            .and_then(std::convert::identity)
            .unwrap_or_else(|| "example.com".to_owned());
        Ok(Self {
            app_name,
            stylized_app_name,
            domain,
        })
    }

    pub fn upgrade(self) -> Result<RequiredConfig, UpgradeError> {
        let development_teams = ios::teams::find_development_teams()
            .map_err(UpgradeError::DeveloperTeamLookupFailed)?;
        Ok(RequiredConfig {
            app_name: self
                .app_name
                .ok_or_else(|| UpgradeError::AppNameNotDetected)?,
            stylized_app_name: self.stylized_app_name,
            domain: self.domain,
            development_team: development_teams
                .get(0)
                .map(|development_team| development_team.id.clone())
                .ok_or_else(|| UpgradeError::DeveloperTeamsEmpty)?,
        })
    }
}
