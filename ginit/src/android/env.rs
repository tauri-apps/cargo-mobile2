use super::ndk;
use crate::util::pure_command::ExplicitEnv;
use std::path::PathBuf;

#[derive(Debug)]
pub enum EnvError {
    BaseEnvError(crate::env::EnvError),
    // TODO: we should be nice and provide a platform-specific suggestion
    AndroidSdkRootNotSet(std::env::VarError),
    AndroidSdkRootNotADir,
    NdkEnvError(ndk::EnvError),
}

#[derive(Debug)]
pub struct Env {
    base: crate::env::Env,
    sdk_root: PathBuf,
    pub ndk: ndk::Env,
}

impl Env {
    pub fn new() -> Result<Self, EnvError> {
        let base = crate::env::Env::new().map_err(EnvError::BaseEnvError)?;
        let sdk_root = std::env::var("ANDROID_SDK_ROOT")
            .map_err(EnvError::AndroidSdkRootNotSet)
            .map(PathBuf::from)
            .and_then(|sdk_root| {
                if sdk_root.is_dir() {
                    Ok(sdk_root)
                } else {
                    Err(EnvError::AndroidSdkRootNotADir)
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
            ndk: ndk::Env::new().map_err(EnvError::NdkEnvError)?,
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
