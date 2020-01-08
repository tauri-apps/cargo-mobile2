use crate::{
    config::{DetectedConfigTrait, RawConfigTrait},
    opts, util,
};
use std::{
    fmt::{self, Display},
    fs::File,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Debug)]
pub enum Error<R: RawConfigTrait> {
    DetectionFailed(<R::Detected as DetectedConfigTrait>::Error),
    UpgradeFailed(R::FromDetectedError),
    PromptFailed(R::FromPromptError),
    SerializationFailed(toml::ser::Error),
    TempWriteFailed(WriteError),
}

impl<R: RawConfigTrait> Display for Error<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DetectionFailed(err) => write!(f, "Failed to detect config: {}", err),
            Self::UpgradeFailed(err) => write!(f, "Failed to upgrade detected config: {}", err),
            Self::PromptFailed(err) => write!(f, "Failed to prompt for config: {}", err),
            Self::SerializationFailed(err) => write!(f, "Failed to serialize config: {}", err),
            Self::TempWriteFailed(err) => write!(f, "Failed to write temp config file: {}", err),
        }
    }
}

pub fn detect_or_prompt<R: RawConfigTrait>(
    interactivity: opts::Interactivity,
    wrapper: &util::TextWrapper,
    name: &str,
) -> Result<(), Error<R>> {
    if !R::is_zst() {
        let detected = R::Detected::new().map_err(Error::DetectionFailed)?;
        let raw = match interactivity {
            opts::Interactivity::Full => {
                R::from_prompt(detected, wrapper).map_err(Error::PromptFailed)
            }
            opts::Interactivity::None => R::from_detected(detected).map_err(Error::UpgradeFailed),
        }?;
        let bytes = toml::to_vec(&raw).map_err(Error::SerializationFailed)?;
        if !bytes.is_empty() {
            write_temp(name, bytes).map_err(Error::TempWriteFailed)
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub enum WriteError {
    DirectoryCreationFailed { path: PathBuf, cause: io::Error },
    CreateFailed { path: PathBuf, cause: io::Error },
    WriteFailed { path: PathBuf, cause: io::Error },
}

impl Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirectoryCreationFailed { path, cause } => {
                write!(f, "Failed to create temp directories {:?}: {}", path, cause)
            }
            Self::CreateFailed { path, cause } => {
                write!(f, "Failed to create temp file {:?}: {}", path, cause)
            }
            Self::WriteFailed { path, cause } => {
                write!(f, "Failed to write temp file {:?}: {}", path, cause)
            }
        }
    }
}

pub fn temp_subdir() -> PathBuf {
    std::env::temp_dir().join("com.brainiumstudios.ginit/plugin-configs")
}

pub fn temp_path(name: &str) -> PathBuf {
    temp_subdir().join(format!("{}.toml", name))
}

fn write_temp(name: &str, bytes: Vec<u8>) -> Result<(), WriteError> {
    let path = temp_path(name);
    {
        let parent = path.parent().unwrap();
        std::fs::create_dir_all(parent).map_err(|cause| WriteError::DirectoryCreationFailed {
            path: parent.to_owned(),
            cause,
        })?;
    }
    log::info!("creating temp config file at {:?}", path);
    let mut file = File::create(&path).map_err(|cause| WriteError::CreateFailed {
        path: path.clone(),
        cause,
    })?;
    file.write_all(&bytes)
        .map_err(|cause| WriteError::WriteFailed {
            path: path.clone(),
            cause,
        })?;
    Ok(())
}
