pub mod config;
pub mod ndk;
pub mod project;
pub mod target;

use std::{ffi::OsStr, path::PathBuf};

#[derive(Debug)]
pub enum EnvError {
    PathNotSet(std::env::VarError),
    TermNotSet(std::env::VarError),
    // TODO: we should be nice and provide a platform-specific suggestion
    AndroidSdkRootNotSet(std::env::VarError),
    AndroidSdkRootNotADir,
    NdkEnvError(ndk::EnvError),
}

#[derive(Debug)]
pub struct Env {
    path: String,
    term: String,
    sdk_root: PathBuf,
    pub ndk: ndk::Env,
}

impl Env {
    pub fn new() -> Result<Self, EnvError> {
        let path = std::env::var("PATH").map_err(EnvError::PathNotSet)?;
        let term = std::env::var("TERM").map_err(EnvError::TermNotSet)?;
        if std::env::var("ANDROID_HOME").is_ok() {
            log::warn!("`ANDROID_HOME` is set, which is deprecated and will be ignored");
        }
        let sdk_root = std::env::var("ANDROID_SDK_ROOT")
            .map_err(EnvError::AndroidSdkRootNotSet)
            .map(PathBuf::from)
            .and_then(|sdk_root| {
                if sdk_root.is_dir() {
                    Ok(sdk_root)
                } else {
                    Err(EnvError::AndroidSdkRootNotADir)
                }
            })?;
        Ok(Self {
            path,
            term,
            sdk_root,
            ndk: ndk::Env::new().map_err(EnvError::NdkEnvError)?,
        })
    }

    pub fn command_env(&self) -> [(&'static str, &OsStr); 4] {
        [
            ("PATH", self.path.as_ref()),
            ("TERM", self.term.as_ref()),
            ("ANDROID_SDK_ROOT", self.sdk_root.as_ref()),
            ("NDK_HOME", self.ndk.home().as_ref()),
        ]
    }
}
