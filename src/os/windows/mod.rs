mod env;
pub(super) mod info;
pub mod ln;

use crate::{env::ExplicitEnv, DuctExpressionExt};
use std::{
    ffi::{OsStr, OsString},
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::Path,
    slice::from_raw_parts,
};
use thiserror::Error;
use windows::{
    core::{self, w, PCWSTR, PWSTR},
    Win32::{
        Foundation::{LocalFree, ERROR_NO_ASSOCIATION, HLOCAL, MAX_PATH},
        System::Registry::HKEY_LOCAL_MACHINE,
        UI::Shell::{
            AssocQueryStringW, CommandLineToArgvW, SHRegGetPathW, ASSOCF_INIT_IGNOREUNKNOWN,
            ASSOCSTR_COMMAND,
        },
    },
};

pub use env::Env;
use which::which;

#[derive(Debug, Error)]
pub enum DetectEditorError {
    #[error("No default editor is set for \".rs\" and \".txt\"")]
    NoDefaultEditorSet,
    #[error("An error occured while calling AssocQueryStringW: {0}")]
    IOError(#[source] std::io::Error),
}

impl From<core::Error> for DetectEditorError {
    fn from(err: core::Error) -> Self {
        Self::IOError(err.into())
    }
}

#[derive(Debug, Error)]
pub enum OpenFileError {
    #[error("Launch Failed: {0}")]
    LaunchFailed(#[source] std::io::Error),
    #[error("An error occured while calling OS API: {0}")]
    IOError(#[source] std::io::Error),
}

pub struct Application {
    argv: Vec<OsString>,
}

const RUST_EXT: PCWSTR = w!(".rs");
const TEXT_EXT: PCWSTR = w!(".txt");

impl Application {
    pub fn detect_editor() -> Result<Self, DetectEditorError> {
        let editor_command =
            Self::detect_associated_command(RUST_EXT).or_else(|err| match err {
                DetectEditorError::NoDefaultEditorSet => Self::detect_associated_command(TEXT_EXT),
                _ => Err(err),
            })?;
        let argv: Vec<_> = NativeArgv::new(&editor_command).into();
        Ok(Self { argv })
    }

    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<(), OpenFileError> {
        let args = self.argv[1..]
            .iter()
            .map(|arg| Self::replace_command_arg(arg, &path.as_ref().as_os_str()))
            .collect::<Vec<_>>();
        duct::cmd(&self.argv[0], args)
            .run_and_detach()
            .map_err(OpenFileError::LaunchFailed)?;
        Ok(())
    }

    fn detect_associated_command(ext: PCWSTR) -> Result<Vec<u16>, DetectEditorError> {
        let mut len: u32 = 0;
        if let Err(e) = unsafe {
            AssocQueryStringW(
                ASSOCF_INIT_IGNOREUNKNOWN,
                ASSOCSTR_COMMAND,
                // In Shlwapi.h, this parameter's type is `LPCWSTR`.
                // So it's not modified actually.
                PCWSTR::from_raw(ext.as_ptr()),
                PCWSTR::null(),
                PWSTR::null(),
                &mut len as _,
            )
            .ok()
        } {
            if e.code().0 == (0x80070000 | ERROR_NO_ASSOCIATION.0) as i32 {
                return Err(DetectEditorError::NoDefaultEditorSet);
            }
            return Err(DetectEditorError::IOError(e.into()));
        }
        let mut command: Vec<u16> = vec![0; len as usize];
        unsafe {
            AssocQueryStringW(
                ASSOCF_INIT_IGNOREUNKNOWN,
                ASSOCSTR_COMMAND,
                // In Shlwapi.h, this parameter's type is `LPCWSTR`.
                // So it's not modified actually.
                PCWSTR::from_raw(RUST_EXT.as_ptr()),
                PCWSTR::null(),
                PWSTR(command.as_mut_ptr()),
                &mut len as _,
            )
            .ok()?;
        }
        Ok(command)
    }

    // Replace %0 or %1 to arg1, and other % is unescape
    fn replace_command_arg(arg: &OsStr, arg1: &OsStr) -> OsString {
        let mut is_percent = false;
        let mut iter = arg.encode_wide();
        let mut buffer = vec![];
        const ZERO: u16 = '0' as u16;
        const ONE: u16 = '1' as u16;
        const TWO: u16 = '2' as u16;
        const NINE: u16 = '9' as u16;
        const PERCENT: u16 = '%' as u16;
        loop {
            match (iter.next(), is_percent) {
                (Some(ZERO..=ONE), true) => {
                    buffer.extend(arg1.encode_wide());
                }
                (Some(TWO..=NINE), true) => {
                    // Nothing to do.
                }
                (Some(PERCENT), false) => {
                    is_percent = true;
                    continue;
                }
                (Some(c), _) => {
                    buffer.push(c);
                }
                (None, _) => break,
            }
            is_percent = false;
        }
        OsString::from_wide(&buffer)
    }
}

pub fn open_file_with(
    application: impl AsRef<OsStr>,
    path: impl AsRef<OsStr>,
    env: &Env,
) -> Result<(), OpenFileError> {
    // In windows, there is no standerd way to find application by name.
    match application.as_ref().to_str() {
        Some("Android Studio") => open_file_with_android_studio(path, env),
        _ => {
            unimplemented!()
        }
    }
}

const ANDROID_STUDIO_UNINSTALL_KEY_PATH: PCWSTR =
    w!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Android Studio");
const ANDROID_STUDIO_UNINSTALLER_VALUE: PCWSTR = w!("UninstallString");
#[cfg(target_pointer_width = "64")]
const STUDIO_EXE_PATH: &str = "bin/studio64.exe";
#[cfg(target_pointer_width = "32")]
const STUDIO_EXE_PATH: &str = "bin/studio.exe";

fn open_file_with_android_studio(path: impl AsRef<OsStr>, env: &Env) -> Result<(), OpenFileError> {
    let mut application_path = which("studio.cmd").unwrap_or_default();
    if !application_path.is_file() {
        let mut buffer = [0; MAX_PATH as usize];
        unsafe {
            SHRegGetPathW(
                HKEY_LOCAL_MACHINE,
                PCWSTR::from_raw(ANDROID_STUDIO_UNINSTALL_KEY_PATH.as_ptr()),
                PCWSTR::from_raw(ANDROID_STUDIO_UNINSTALLER_VALUE.as_ptr()),
                &mut buffer,
                0,
            )
            .ok()
            .map_err(|e| OpenFileError::IOError(e.into()))?
        };
        let len = NullTerminatedWTF16Iterator(buffer.as_ptr()).count();
        let uninstaller_path = OsString::from_wide(&buffer[..len]);
        application_path = Path::new(&uninstaller_path)
            .parent()
            .expect("Failed to get Android Studio uninstaller's parent path")
            .join(STUDIO_EXE_PATH);
    }
    duct::cmd(
        application_path,
        [
            dunce::canonicalize(Path::new(path.as_ref()))
                .expect("Failed to canonicalize file path"),
        ],
    )
    .vars(env.explicit_env())
    .run_and_detach()
    .map_err(OpenFileError::LaunchFailed)?;
    Ok(())
}

pub fn command_path(name: &str) -> std::io::Result<std::process::Output> {
    duct::cmd("where.exe", [name]).run()
}

struct NativeArgv {
    argv: *mut PWSTR,
    len: i32,
}

impl NativeArgv {
    // The buffer must be null terminated.
    fn new(buffer: &[u16]) -> Self {
        let mut len = 0;
        // In shellap.h, lpcmdline's type is `LPCWSTR`.
        // So it's not modified actually.
        let argv = unsafe { CommandLineToArgvW(PCWSTR::from_raw(buffer.as_ptr()), &mut len as _) };
        Self { argv, len }
    }
}

impl Drop for NativeArgv {
    fn drop(&mut self) {
        let _ = unsafe { LocalFree(HLOCAL(self.argv as _)) };
    }
}

impl From<NativeArgv> for Vec<OsString> {
    fn from(native_argv: NativeArgv) -> Self {
        let mut argv = Vec::with_capacity(native_argv.len as usize);
        let argv_slice = unsafe { from_raw_parts(native_argv.argv, native_argv.len as _) };
        for pwstr in argv_slice {
            let len = NullTerminatedWTF16Iterator(pwstr.0).count();
            let arg = OsString::from_wide(unsafe { std::slice::from_raw_parts(pwstr.0, len) });
            argv.push(arg);
        }
        argv
    }
}

struct NullTerminatedWTF16Iterator(*const u16);

impl Iterator for NullTerminatedWTF16Iterator {
    type Item = u16;
    fn next(&mut self) -> Option<Self::Item> {
        match unsafe { *self.0 } {
            0 => None,
            c => {
                self.0 = unsafe { self.0.offset(1) };
                Some(c)
            }
        }
    }
}

// Directly invoking code.cmd behaves strangely.
// For example, if running `cargo mobile new foo` in C:\Users\MyHome,
// %~dp0 will expand to C:\Users\MyHome\foo in code.cmd, which is completely broken.
// Running it through powershell.exe does not have this problem.
pub fn code_command() -> duct::Expression {
    duct::cmd!("code.cmd")
}

pub fn replace_path_separator(path: OsString) -> OsString {
    let buf = path
        .encode_wide()
        .map(|c| if c == '\\' as u16 { '/' as u16 } else { c })
        .collect::<Vec<_>>();
    OsString::from_wide(&buf)
}

pub mod consts {
    pub const CLANG: &str = "clang.cmd";
    pub const CLANGXX: &str = "clang++.cmd";
    pub const LD: &str = "ld.exe";
    pub const AR: &str = "ar.exe";
    pub const READELF: &str = "readelf.exe";
    pub const NDK_STACK: &str = "ndk-stack.cmd";
}
