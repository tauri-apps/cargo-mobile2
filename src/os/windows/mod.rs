mod env;
pub(super) mod info;
pub mod ln;

use std::{
    ffi::{OsStr, OsString},
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::Path,
    slice::from_raw_parts,
};
use thiserror::Error;
use windows::{
    runtime,
    Win32::{
        Foundation::{ERROR_NO_ASSOCIATION, ERROR_SUCCESS, MAX_PATH, PWSTR},
        System::{Memory::LocalFree, Registry::HKEY_LOCAL_MACHINE},
        UI::Shell::{
            AssocQueryStringW, CommandLineToArgvW, SHRegGetPathW, ASSOCF_INIT_IGNOREUNKNOWN,
            ASSOCSTR_COMMAND,
        },
    },
};

pub use env::Env;

#[derive(Debug, Error)]
pub enum DetectEditorError {
    #[error("No default editor is set: AssocQueryStringW for \".rs\" and \".txt\" both failed")]
    NoDefaultEditorSet,
    #[error("An error occured while calling AssocQueryStringW: {0}")]
    IOError(#[source] std::io::Error),
}

impl From<runtime::Error> for DetectEditorError {
    fn from(err: runtime::Error) -> Self {
        Self::IOError(err.into())
    }
}

#[derive(Debug, Error)]
pub enum OpenFileError {
    #[error("Launch Failed: {0}")]
    LaunchFailed(#[source] bossy::Error),
    #[error("An error occured while calling OS API: {0}")]
    IOError(#[source] std::io::Error),
}

pub struct Application {
    argv: Vec<OsString>,
}

const RUST_EXT: &[u16] = const_utf16::encode_null_terminated!(".rs");
const TEXT_EXT: &[u16] = const_utf16::encode_null_terminated!(".txt");

impl Application {
    pub fn detect_editor() -> Result<Self, DetectEditorError> {
        let editor_command = Self::detect_associated_command(RUST_EXT).or_else(|e| match e {
            DetectEditorError::NoDefaultEditorSet => Self::detect_associated_command(TEXT_EXT),
            err => Err(err),
        })?;
        let argv: Vec<_> = NativeArgv::new(&editor_command).into();

        Ok(Self { argv })
    }

    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<(), OpenFileError> {
        let args = self.argv[1..]
            .iter()
            .map(|arg| Self::replace_command_arg(arg, &path.as_ref().as_os_str()))
            .collect::<Vec<_>>();
        bossy::Command::impure(&self.argv[0])
            .add_args(&args)
            .run_and_detach()
            .map_err(OpenFileError::LaunchFailed)
    }

    fn detect_associated_command(ext: &[u16]) -> Result<Vec<u16>, DetectEditorError> {
        let mut len: u32 = 0;
        if let Err(e) = unsafe {
            AssocQueryStringW(
                ASSOCF_INIT_IGNOREUNKNOWN as u32,
                ASSOCSTR_COMMAND,
                // In Shlwapi.h, this parameter's type is `LPCWSTR`.
                // So it's not modified actually.
                PWSTR(ext.as_ptr() as _),
                PWSTR::default(),
                PWSTR::default(),
                &mut len as _,
            )
        } {
            if e.code().0 == 0x80070000 | ERROR_NO_ASSOCIATION.0 {
                return Err(DetectEditorError::NoDefaultEditorSet);
            }
            return Err(DetectEditorError::IOError(e.into()));
        }
        let mut command: Vec<u16> = vec![0; len as usize];
        unsafe {
            AssocQueryStringW(
                ASSOCF_INIT_IGNOREUNKNOWN as u32,
                ASSOCSTR_COMMAND,
                // In Shlwapi.h, this parameter's type is `LPCWSTR`.
                // So it's not modified actually.
                PWSTR(RUST_EXT.as_ptr() as _),
                PWSTR::default(),
                PWSTR(command.as_mut_ptr()),
                &mut len as _,
            )
        }?;
        return Ok(command);
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
) -> Result<(), OpenFileError> {
    // In windows, there is no standerd way to find application by name.
    match application.as_ref().to_str() {
        Some("Android Studio") => open_file_with_android_studio(path),
        _ => {
            unimplemented!()
        }
    }
}

const ANDROID_STUDIO_UNINSTALL_KEY_PATH: &[u16] = const_utf16::encode_null_terminated!(
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Android Studio"
);
const ANDROID_STUDIO_UNINSTALLER_VALUE: &[u16] =
    const_utf16::encode_null_terminated!("UninstallString");
#[cfg(target_pointer_width = "64")]
const STUDIO_EXE_PATH: &str = "bin/studio64.exe";
#[cfg(target_pointer_width = "32")]
const STUDIO_EXE_PATH: &str = "bin/studio.exe";

fn open_file_with_android_studio(path: impl AsRef<OsStr>) -> Result<(), OpenFileError> {
    let mut buffer = [0; MAX_PATH as usize];
    let lstatus = unsafe {
        SHRegGetPathW(
            HKEY_LOCAL_MACHINE,
            PWSTR(ANDROID_STUDIO_UNINSTALL_KEY_PATH.as_ptr() as _),
            PWSTR(ANDROID_STUDIO_UNINSTALLER_VALUE.as_ptr() as _),
            PWSTR(buffer.as_mut_ptr()),
            0,
        )
    };
    if lstatus.0 as u32 != ERROR_SUCCESS.0 {
        return Err(OpenFileError::IOError(runtime::Error::from_win32().into()));
    }
    let len = NullTerminatedWTF16Iterator(buffer.as_ptr()).count();
    let uninstaller_path = OsString::from_wide(&buffer[..len]);
    let application_path = Path::new(&uninstaller_path)
        .parent()
        .expect("failed to getAndroid Studio uninstaller's parent path")
        .join(STUDIO_EXE_PATH);
    bossy::Command::impure(application_path)
        .add_arg(
            dunce::canonicalize(Path::new(path.as_ref()))
                .expect("Failed to canonicalize file path"),
        )
        .run_and_wait()
        .map_err(OpenFileError::LaunchFailed)?;
    Ok(())
}

pub fn command_path(name: &str) -> bossy::Result<bossy::Output> {
    bossy::Command::impure("where.exe")
        .add_arg(name)
        .run_and_wait_for_output()
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
        let argv = unsafe { CommandLineToArgvW(PWSTR(buffer.as_ptr() as _), &mut len as _) };
        Self { argv, len }
    }
}

impl Drop for NativeArgv {
    fn drop(&mut self) {
        unsafe { LocalFree(self.argv as _) };
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
pub fn code_command() -> bossy::Command {
    bossy::Command::impure("powershell.exe").with_args(&["-Command", "code"])
}

pub fn gradlew_command(project_dir: impl AsRef<OsStr>) -> bossy::Command {
    // Path without verbatim prefix.
    let project_dir = dunce::canonicalize(Path::new(project_dir.as_ref()))
        .expect("Failed to canonicalize project dir");
    let gradlew_path = project_dir.join("gradlew.bat");
    bossy::Command::impure(&gradlew_path)
        .with_arg("--project-dir")
        .with_arg(&project_dir)
}

pub fn replace_path_separator(path: OsString) -> OsString {
    let buf = path
        .encode_wide()
        .map(|c| if c == '\\' as u16 { '/' as u16 } else { c })
        .collect::<Vec<_>>();
    OsString::from_wide(&buf)
}

pub mod consts {
    pub const AR: &str = "ar.exe";
    pub const CLANG: &str = "clang.cmd";
    pub const CLANGXX: &str = "clang++.cmd";
    pub const LD: &str = "ld.exe";
    pub const READELF: &str = "readelf.exe";
    pub const NDK_STACK: &str = "ndk-stack.cmd";
}
