use super::ndk;
use crate::{
    env::{Env as CoreEnv, Error as CoreError, ExplicitEnv},
    util::cli::{Report, Reportable},
};
use std::{
    fmt::{self, Display},
    path::PathBuf,
};

#[derive(Debug)]
pub enum Error {
    CoreEnvError(CoreError),
    // TODO: we should be nice and provide a platform-specific suggestion
    AndroidSdkRootNotSet(std::env::VarError),
    AndroidSdkRootNotADir,
    NdkEnvError(ndk::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoreEnvError(err) => write!(f, "{}", err),
            Self::AndroidSdkRootNotSet(err) => {
                write!(f, "Have you installed the Android SDK? The `ANDROID_SDK_ROOT` environment variable isn't set, and is required: {}", err)
            }
            Self::AndroidSdkRootNotADir => write!(
                f,
                "Have you installed the Android SDK? The `ANDROID_SDK_ROOT` environment variable is set, but doesn't point to an existing directory."
            ),
            Self::NdkEnvError(err) => write!(f, "{}", err),
        }
    }
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::CoreEnvError(err) => err.report(),
            Self::NdkEnvError(err) => err.report(),
            _ => Report::error("Failed to initialize Android environment", self),
        }
    }
}

impl Error {
    pub fn sdk_or_ndk_issue(&self) -> bool {
        !matches!(self, Self::CoreEnvError(_))
    }
}

#[derive(Debug)]
pub struct Env {
    base: CoreEnv,
    sdk_root: PathBuf,
    pub ndk: ndk::Env,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        let base = CoreEnv::new().map_err(Error::CoreEnvError)?;
        let sdk_root = std::env::var("ANDROID_SDK_ROOT")
            .map_err(Error::AndroidSdkRootNotSet)
            .map(PathBuf::from)
            .and_then(|sdk_root| {
                if sdk_root.is_dir() {
                    Ok(sdk_root)
                } else {
                    Err(Error::AndroidSdkRootNotADir)
                }
            })
            .or_else(|err| {
                if let Some(android_home) = std::env::var("ANDROID_HOME")
                    .ok()
                    .map(PathBuf::from)
                    .filter(|android_home| android_home.is_dir())
                {
                    log::warn!("`ANDROID_SDK_ROOT` isn't set; falling back to `ANDROID_HOME`, which is deprecated");
                    Ok(android_home)
                } else {
                    Err(err)
                }
            })
            .or_else(|err| {
                if let Some(android_home) = std::env::var("ANDROID_HOME")
                    .ok()
                    .map(PathBuf::from)
                    .filter(|android_home| android_home.is_dir())
                {
                    log::warn!("`ANDROID_SDK_ROOT` isn't set; falling back to `ANDROID_HOME`, which is deprecated");
                    Ok(android_home)
                } else {
                    Err(err)
                }
            })?;
        Ok(Self {
            base,
            sdk_root,
            ndk: ndk::Env::new().map_err(Error::NdkEnvError)?,
        })
    }

    pub fn path(&self) -> &str {
        self.base.path()
    }
}

impl ExplicitEnv for Env {
    fn explicit_env(&self) -> Vec<(&str, &std::ffi::OsStr)> {
        let mut envs = self.base.explicit_env();
        envs.extend(&[
            ("ANDROID_SDK_ROOT", self.sdk_root.as_ref()),
            ("NDK_HOME", self.ndk.home().as_ref()),
        ]);
        envs
    }
}
