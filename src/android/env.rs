use super::{
    ndk,
    source_props::{self, SourceProps},
};
use crate::{
    env::{Error as CoreError, ExplicitEnv},
    os::Env as CoreEnv,
    util::cli::{Report, Reportable},
};
use std::{collections::HashMap, ffi::OsString, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    CoreEnvError(#[from] CoreError),
    // TODO: we should be nice and provide a platform-specific suggestion
    #[error("Have you installed the Android SDK? The `ANDROID_HOME` environment variable isn't set, and is required: {0}")]
    AndroidHomeNotSet(#[from] std::env::VarError),
    #[error("Have you installed the Android SDK? The `ANDROID_HOME` environment variable is set, but doesn't point to an existing directory.")]
    AndroidHomeNotADir,
    #[error(transparent)]
    NdkEnvError(#[from] ndk::Error),
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

#[derive(Debug, Clone)]
pub struct Env {
    pub base: CoreEnv,
    android_home: PathBuf,
    pub ndk: ndk::Env,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        Self::from_env(CoreEnv::new()?)
    }

    pub fn from_env(base: CoreEnv) -> Result<Self, Error> {
        let android_home = std::env::var("ANDROID_HOME")
            .map_err(Error::AndroidHomeNotSet)
            .map(PathBuf::from)
            .and_then(|android_home| {
                if android_home.is_dir() {
                    Ok(android_home)
                } else {
                    Err(Error::AndroidHomeNotADir)
                }
            })
            .or_else(|err| {
                if let Some(sdk_root) = std::env::var("ANDROID_SDK_ROOT")
                    .ok()
                    .map(PathBuf::from)
                    .filter(|sdk_root| sdk_root.is_dir())
                {
                    log::warn!("`ANDROID_HOME` isn't set; falling back to `ANDROID_SDK_ROOT`, which is deprecated");
                    Ok(sdk_root)
                } else {
                    Err(err)
                }
            })
            .or_else(|err| {
                if let Some(sdk_root) = std::env::var("ANDROID_SDK_ROOT")
                    .ok()
                    .map(PathBuf::from)
                    .filter(|sdk_root| sdk_root.is_dir())
                {
                    log::warn!("`ANDROID_HOME` isn't set; falling back to `ANDROID_SDK_ROOT`, which is deprecated");
                    Ok(sdk_root)
                } else {
                    Err(err)
                }
            })?;
        Ok(Self {
            base,
            android_home,
            ndk: ndk::Env::new()?,
        })
    }

    pub fn path(&self) -> &OsString {
        self.base.path()
    }

    pub fn android_home(&self) -> &str {
        self.android_home.as_path().to_str().unwrap()
    }

    pub fn platform_tools_path(&self) -> PathBuf {
        PathBuf::from(&self.android_home).join("platform-tools")
    }

    pub fn sdk_version(&self) -> Result<source_props::Revision, source_props::Error> {
        SourceProps::from_path(self.platform_tools_path().join("source.properties"))
            .map(|props| props.pkg.revision)
    }
}

impl ExplicitEnv for Env {
    fn explicit_env(&self) -> HashMap<String, OsString> {
        let mut envs = self.base.explicit_env();
        envs.insert(
            "ANDROID_HOME".into(),
            self.android_home.as_os_str().to_os_string(),
        );
        envs.insert(
            "NDK_HOME".into(),
            self.ndk.home().as_os_str().to_os_string(),
        );
        envs
    }
}
