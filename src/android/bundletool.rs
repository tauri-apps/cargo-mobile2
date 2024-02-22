#[cfg(not(target_os = "macos"))]
use crate::util;
use crate::util::cli::{Report, Reportable};
#[cfg(not(target_os = "macos"))]
use std::path::PathBuf;
use thiserror::Error;

use crate::DuctExpressionExt;

#[cfg(not(target_os = "macos"))]
pub const BUNDLE_TOOL_JAR_INFO: BundletoolJarInfo = BundletoolJarInfo { version: "1.8.0" };

#[cfg(not(target_os = "macos"))]
pub struct BundletoolJarInfo {
    version: &'static str,
}

#[cfg(not(target_os = "macos"))]
impl BundletoolJarInfo {
    fn file_name(&self) -> String {
        format!("bundletool-all-{}.jar", self.version)
    }

    fn installation_path(&self) -> PathBuf {
        util::tools_dir()
            .map(|tools_dir| tools_dir.join(self.file_name()))
            .unwrap()
    }

    fn download_url(&self) -> String {
        format!(
            "https://github.com/google/bundletool/releases/download/{}/{}",
            self.version,
            self.file_name()
        )
    }

    fn run_command(&self) -> duct::Expression {
        let installation_path = self.installation_path();
        duct::cmd("java", ["-jar"])
            .dup_stdio()
            .before_spawn(move |cmd| {
                cmd.arg(&installation_path);
                Ok(())
            })
    }
}

pub fn command() -> duct::Expression {
    #[cfg(not(target_os = "macos"))]
    {
        BUNDLE_TOOL_JAR_INFO.run_command()
    }
    #[cfg(target_os = "macos")]
    {
        duct::cmd!("bundletool").dup_stdio()
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug, Error)]
#[error(transparent)]
pub struct InstallError(crate::apple::deps::Error);

#[cfg(target_os = "macos")]
impl Reportable for InstallError {
    fn report(&self) -> Report {
        Report::error("Failed to install `bundletool`", &self.0)
    }
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Error)]
pub enum InstallError {
    #[error("Failed to download `bundletool`: {0}")]
    Download(Box<ureq::Error>),
    #[error("Failed to create bundletool.jar at {path}: {cause}")]
    JarFileCreation {
        path: PathBuf,
        cause: std::io::Error,
    },
    #[error("Failed to copy content into bundletool.jar at {path}: {cause}")]
    CopyToFile {
        path: PathBuf,
        cause: std::io::Error,
    },
}

#[cfg(not(target_os = "macos"))]
impl Reportable for InstallError {
    fn report(&self) -> Report {
        match self {
            Self::Download(err) => Report::error("Failed to download `bundletool`", err),
            Self::JarFileCreation { path, cause } => Report::error(
                format!("Failed to create bundletool.jar at {:?}", path),
                cause,
            ),
            Self::CopyToFile { path, cause } => Report::error(
                format!("Failed to copy content into bundletool.jar at {:?}", path),
                cause,
            ),
        }
    }
}

pub fn install(reinstall_deps: bool) -> Result<(), InstallError> {
    #[cfg(not(target_os = "macos"))]
    {
        let jar_path = BUNDLE_TOOL_JAR_INFO.installation_path();
        if !jar_path.exists() || reinstall_deps {
            let response = ureq::get(&BUNDLE_TOOL_JAR_INFO.download_url())
                .call()
                .map_err(Box::new)
                .map_err(InstallError::Download)?;
            let tools_dir = util::tools_dir().unwrap();
            std::fs::create_dir_all(&tools_dir).map_err(|cause| InstallError::JarFileCreation {
                path: tools_dir,
                cause,
            })?;
            let mut out = std::fs::File::create(&jar_path).map_err(|cause| {
                InstallError::JarFileCreation {
                    path: jar_path.clone(),
                    cause,
                }
            })?;
            std::io::copy(&mut response.into_reader(), &mut out).map_err(|cause| {
                InstallError::CopyToFile {
                    path: jar_path,
                    cause,
                }
            })?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        use crate::apple::deps::{GemCache, PackageSpec};
        PackageSpec::brew("bundletool")
            .install(reinstall_deps, &mut GemCache::new())
            .map_err(InstallError)?;
    }
    Ok(())
}
