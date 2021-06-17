use super::{
    ndk,
    source_props::{self, SourceProps},
};
use crate::{
    env::{Env as CoreEnv, Error as CoreError, ExplicitEnv},
    util::cli::{Report, Reportable},
};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    CoreEnvError(#[from] CoreError),
    // TODO: we should be nice and provide a platform-specific suggestion
    #[error("Have you installed the Android SDK? The `ANDROID_SDK_ROOT` environment variable isn't set, and is required: {0}")]
    AndroidSdkRootNotSet(#[from] std::env::VarError),
    #[error("Have you installed the Android SDK? The `ANDROID_SDK_ROOT` environment variable is set, but doesn't point to an existing directory.")]
    AndroidSdkRootNotADir,
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

#[derive(Debug)]
pub struct Env {
    base: CoreEnv,
    sdk_root: PathBuf,
    pub ndk: ndk::Env,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        Self::from_env(CoreEnv::new()?)
    }

    pub fn from_env(base: CoreEnv) -> Result<Self, Error> {
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
            ndk: ndk::Env::new()?,
        })
    }

    pub fn path(&self) -> &str {
        self.base.path()
    }

    pub fn sdk_root(&self) -> &str {
        self.sdk_root.as_path().to_str().unwrap()
    }

    pub fn sdk_version(&self) -> Result<source_props::Revision, source_props::Error> {
        SourceProps::from_path(Path::new(self.sdk_root()).join("tools/source.properties"))
            .map(|props| props.pkg.revision)
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
