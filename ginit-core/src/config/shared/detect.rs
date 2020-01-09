use crate::{
    config::{app_name, DetectedConfigTrait},
    util::COMMON_EMAIL_PROVIDERS,
};
use heck::{KebabCase as _, TitleCase as _};
use into_result::{command::CommandError, IntoResult as _};
use std::{
    env,
    fmt::{self, Display},
    io,
    path::PathBuf,
    process::Command,
};

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
pub enum DetectError {
    CurrentDirFailed(io::Error),
    CurrentDirHasNoName(PathBuf),
    CurrentDirInvalidUtf8(PathBuf),
}

impl Display for DetectError {
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
pub struct Detected {
    pub app_name: Option<String>,
    pub stylized_app_name: String,
    pub domain: String,
}

impl DetectedConfigTrait for Detected {
    type Error = DetectError;
    fn new() -> Result<Self, Self::Error> {
        let cwd = env::current_dir().map_err(DetectError::CurrentDirFailed)?;
        let dir_name = cwd
            .file_name()
            .ok_or_else(|| DetectError::CurrentDirHasNoName(cwd.clone()))?;
        let dir_name_str = dir_name
            .to_str()
            .ok_or_else(|| DetectError::CurrentDirInvalidUtf8(cwd.clone()))?;
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
}
