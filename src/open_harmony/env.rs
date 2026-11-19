use crate::{
    env::{Error as CoreError, ExplicitEnv},
    os::Env as CoreEnv,
    util::cli::{Report, Reportable},
};
use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    CoreEnvError(#[from] CoreError),
    // TODO: we should be nice and provide a platform-specific suggestion
    #[error("Have you installed the OpenHarmony SDK? The `OHOS_HOME` environment variable is set, but doesn't point to an existing directory.")]
    OhosHomeNotADir,
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::CoreEnvError(err) => err.report(),
            _ => Report::error("Failed to initialize OpenHarmony environment", self),
        }
    }
}

impl Error {
    pub fn sdk_issue(&self) -> bool {
        !matches!(self, Self::CoreEnvError(_))
    }
}

#[derive(Debug, Clone)]
pub struct Env {
    pub base: CoreEnv,
    ohos_home: PathBuf,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        Self::from_env(CoreEnv::new()?)
    }

    pub fn from_env(base: CoreEnv) -> Result<Self, Error> {
        let ohos_home = std::env::var("OHOS_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                if cfg!(target_os = "macos") {
                    PathBuf::from(
                        "/Applications/DevEco-Studio.app/Contents/sdk/default/openharmony",
                    )
                } else {
                    std::env::var("DEV_ECO_STUDIO_INSTALL_PATH")
                        .map(PathBuf::from)
                        .unwrap_or_else(|_| {
                            PathBuf::from("C:\\Program Files\\Huawei\\DevEco Studio")
                        })
                        .join("sdk")
                        .join("default")
                        .join("openharmony")
                }
            });

        if ohos_home.is_dir() {
            Ok(Self { base, ohos_home })
        } else {
            Err(Error::OhosHomeNotADir)
        }
    }

    pub fn path(&self) -> &OsString {
        self.base.path()
    }

    pub fn ohos_home(&self) -> &Path {
        &self.ohos_home
    }

    pub fn toolchains_path(&self) -> PathBuf {
        PathBuf::from(&self.ohos_home).join("toolchains")
    }
}

impl ExplicitEnv for Env {
    fn explicit_env(&self) -> HashMap<String, OsString> {
        let mut envs = self.base.explicit_env();
        envs.insert(
            "OHOS_HOME".into(),
            self.ohos_home.as_os_str().to_os_string(),
        );
        envs.insert(
            "OHOS_NDK_HOME".into(),
            self.ohos_home.as_os_str().to_os_string(),
        );
        envs.insert(
            "OHOS_BASE_SDK_HOME".into(),
            self.ohos_home.as_os_str().to_os_string(),
        );
        // seems like only Linux requires this, but let's see
        // OHOS_HOME is /path/to/sdk/default/openharmony/18, we want /path/to/sdk for DEVECO_SDK_HOME
        envs.insert(
            "DEVECO_SDK_HOME".into(),
            self.ohos_home
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .as_os_str()
                .to_os_string(),
        );
        envs
    }
}
