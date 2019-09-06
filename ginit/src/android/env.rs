use super::ndk;
use crate::util::pure_command::ExplicitEnv;
use std::{fmt, path::PathBuf};

#[derive(Debug)]
pub enum Error {
    BaseEnvError(crate::env::Error),
    // TODO: we should be nice and provide a platform-specific suggestion
    AndroidSdkRootNotSet(std::env::VarError),
    AndroidSdkRootNotADir,
    NdkEnvError(ndk::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BaseEnvError(err) => write!(f, "{}", err),
            Error::AndroidSdkRootNotSet(err) => {
                write!(f, "The `ANDROID_SDK_ROOT` environment variable isn't set, and is required: {}", err)
            }
            Error::AndroidSdkRootNotADir => write!(
                f,
                "The `ANDROID_SDK_ROOT` environment variable is set, but doesn't point to an existing directory."
            ),
            Error::NdkEnvError(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug)]
pub struct Env {
    base: crate::env::Env,
    sdk_root: PathBuf,
    pub ndk: ndk::Env,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        let base = crate::env::Env::new().map_err(Error::BaseEnvError)?;
        if std::env::var("ANDROID_HOME").is_ok() {
            log::warn!("`ANDROID_HOME` is set, which is deprecated and will be ignored");
        }
        let sdk_root = std::env::var("ANDROID_SDK_ROOT")
            .map_err(Error::AndroidSdkRootNotSet)
            .map(PathBuf::from)
            .and_then(|sdk_root| {
                if sdk_root.is_dir() {
                    Ok(sdk_root)
                } else {
                    Err(Error::AndroidSdkRootNotADir)
                }
            })?;
        Ok(Self {
            base,
            sdk_root,
            ndk: ndk::Env::new().map_err(Error::NdkEnvError)?,
        })
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
