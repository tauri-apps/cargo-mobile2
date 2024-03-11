mod ffi;
pub(super) mod info;

use crate::{env::ExplicitEnv, DuctExpressionExt};
use core_foundation::{
    array::CFArray,
    base::{OSStatus, TCFType},
    error::{CFError, CFErrorRef},
    string::{CFString, CFStringRef},
    url::CFURL,
};
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    ptr,
};
use thiserror::Error;

pub use crate::{env::Env, util::ln};

// This can hopefully be relied upon... https://stackoverflow.com/q/8003919
static RUST_UTI: &str = "dyn.ah62d4rv4ge81e62";

#[derive(Debug, Error)]
pub enum DetectEditorError {
    #[error(transparent)]
    LookupFailed(CFError),
}

#[derive(Debug, Error)]
pub enum OpenFileError {
    #[error("Failed to convert path {path} into a `CFURL`.")]
    PathToUrlFailed { path: PathBuf },
    #[error("Status code {0}")]
    LaunchFailed(OSStatus),
    #[error("Launch failed: {0}")]
    DuctLaunchFailed(std::io::Error),
}

#[derive(Debug)]
pub struct Application {
    url: CFURL,
}

impl Application {
    pub fn detect_editor() -> Result<Self, DetectEditorError> {
        unsafe fn inner(uti: CFStringRef) -> Result<CFURL, CFError> {
            let mut err: CFErrorRef = ptr::null_mut();
            let out_url =
                ffi::LSCopyDefaultApplicationURLForContentType(uti, ffi::kLSRolesEditor, &mut err);
            if out_url.is_null() {
                Err(TCFType::wrap_under_create_rule(err))
            } else {
                Ok(TCFType::wrap_under_create_rule(out_url))
            }
        }
        let uti = CFString::from_static_string(RUST_UTI);
        let url =
            unsafe { inner(uti.as_concrete_TypeRef()) }.map_err(DetectEditorError::LookupFailed)?;
        Ok(Self { url })
    }

    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<(), OpenFileError> {
        let path = path.as_ref();
        let item_url = CFURL::from_path(path, path.is_dir()).ok_or_else(|| {
            OpenFileError::PathToUrlFailed {
                path: path.to_owned(),
            }
        })?;
        let items = CFArray::from_CFTypes(&[item_url]);
        let spec = ffi::LSLaunchURLSpec::new(
            self.url.as_concrete_TypeRef(),
            items.as_concrete_TypeRef(),
            ffi::kLSLaunchDefaults,
        );
        let status = unsafe { ffi::LSOpenFromURLSpec(&spec, ptr::null_mut()) };
        if status == 0 {
            Ok(())
        } else {
            Err(OpenFileError::LaunchFailed(status))
        }
    }
}

pub fn open_file_with(
    application: impl AsRef<OsStr>,
    path: impl AsRef<OsStr>,
    env: &Env,
) -> Result<(), OpenFileError> {
    let application = application.as_ref().to_os_string();
    let path = path.as_ref().to_os_string();
    duct::cmd("open", ["-a"])
        .before_spawn(move |cmd| {
            cmd.arg(&application).arg(&path);
            Ok(())
        })
        .vars(env.explicit_env())
        .run_and_detach()
        .map_err(OpenFileError::DuctLaunchFailed)?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn command_path(name: &str) -> std::io::Result<std::process::Output> {
    duct::cmd("command", ["-v", name]).dup_stdio().run()
}

pub fn code_command() -> duct::Expression {
    duct::cmd!("code")
}

pub fn replace_path_separator(path: OsString) -> OsString {
    path
}

pub fn open_in_xcode(path: impl AsRef<OsStr>) -> Result<(), OpenFileError> {
    duct::cmd("xed", [path.as_ref()])
        .run_and_detach()
        .map_err(OpenFileError::DuctLaunchFailed)?;
    Ok(())
}

pub mod consts {
    pub const CLANG: &str = "clang";
    pub const CLANGXX: &str = "clang++";
    pub const AR: &str = "ar";
    pub const LD: &str = "ld";
    pub const READELF: &str = "readelf";
    pub const NDK_STACK: &str = "ndk-stack";
}
