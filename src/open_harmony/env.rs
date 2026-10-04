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
    #[error("Cannot infer DEVECO_SDK_HOME from OHOS_HOME; set DEVECO_SDK_HOME explicitly")]
    DeVeCoSdkHomeUnknown,
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
    deveco_sdk_home: PathBuf,
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
            let deveco_sdk_home = sdk_home(&ohos_home, std::env::var_os("DEVECO_SDK_HOME"))
                .ok_or(Error::DeVeCoSdkHomeUnknown)?;
            Ok(Self {
                base,
                ohos_home,
                deveco_sdk_home,
            })
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
        envs.insert(
            "DEVECO_SDK_HOME".into(),
            self.deveco_sdk_home.as_os_str().to_os_string(),
        );
        envs
    }
}

// DevEco packages use sdk/default/openharmony, while standalone SDKs may add an API directory.
fn sdk_home(ohos_home: &Path, explicit: Option<OsString>) -> Option<PathBuf> {
    if let Some(explicit) = explicit {
        return Some(PathBuf::from(explicit));
    }
    let openharmony = if ohos_home.file_name()? == "openharmony" {
        ohos_home
    } else {
        ohos_home.parent()?
    };
    Some(openharmony.parent()?.parent()?.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_home_supports_both_sdk_layouts() {
        let root = PathBuf::from("tools").join("sdk");
        let ohos = root.join("default").join("openharmony");
        assert_eq!(sdk_home(&ohos, None), Some(root.clone()));
        assert_eq!(sdk_home(&ohos.join("18"), None), Some(root));
    }

    #[test]
    fn explicit_sdk_home_takes_precedence() {
        let explicit = PathBuf::from("custom-sdk");
        assert_eq!(
            sdk_home(
                Path::new("openharmony"),
                Some(explicit.clone().into_os_string())
            ),
            Some(explicit),
        );
    }

    #[test]
    fn shallow_sdk_path_requires_explicit_home() {
        assert_eq!(sdk_home(Path::new("openharmony"), None), None);
    }
}
