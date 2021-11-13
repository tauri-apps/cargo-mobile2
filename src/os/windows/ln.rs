use crate::util::ln::{Clobber, Error, ErrorCause, LinkType, TargetStyle};
use std::{borrow::Cow, fs::remove_file, os::windows::ffi::OsStrExt, path::Path};
use windows::{
    runtime,
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, BOOLEAN, ERROR_MORE_DATA, HANDLE, INVALID_HANDLE_VALUE,
            PWSTR,
        },
        Storage::FileSystem::{
            CreateFileW, CreateSymbolicLinkW, GetFileAttributesW, FILE_ACCESS_FLAGS,
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_DELETE_ON_CLOSE,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, OPEN_EXISTING, REPARSE_GUID_DATA_BUFFER,
            SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE,
        },
        System::{
            Ioctl::FSCTL_GET_REPARSE_POINT,
            SystemServices::{GENERIC_READ, IO_REPARSE_TAG_SYMLINK},
            IO::DeviceIoControl,
        },
    },
};

pub fn force_symlink(
    source: impl AsRef<Path>,
    target: impl AsRef<Path>,
    target_style: TargetStyle,
) -> Result<(), Error> {
    let (source, target) = (source.as_ref(), target.as_ref());
    let error = |cause: ErrorCause| {
        Error::new(
            LinkType::Symbolic,
            Clobber::FileOnly,
            source.to_owned(),
            target.to_owned(),
            target_style,
            cause,
        )
    };
    let target = if target_style == TargetStyle::Directory {
        let file_name = if let Some(file_name) = source.file_name() {
            file_name
        } else {
            return Err(error(ErrorCause::MissingFileName));
        };
        Cow::Owned(target.join(file_name))
    } else {
        Cow::Borrowed(target)
    };
    let target_wtf16 = target
        .as_os_str()
        .encode_wide()
        .chain([0])
        .collect::<Vec<_>>();
    if is_symlink(&target_wtf16) {
        delete_symlink(&target_wtf16).map_err(|err| error(ErrorCause::IOError(err.into())))?;
    } else if target.is_file() {
        remove_file(&target).map_err(|err| error(ErrorCause::IOError(err)))?;
    }
    let source_wtf16 = source
        .as_os_str()
        .encode_wide()
        .chain([0])
        .collect::<Vec<_>>();
    create_symlink(&target_wtf16, &source_wtf16)
        .map_err(|err| error(ErrorCause::IOError(err.into())))?;
    Ok(())
}

pub fn force_symlink_relative(
    abs_source: impl AsRef<Path>,
    abs_target: impl AsRef<Path>,
    target_style: TargetStyle,
) -> Result<(), Error> {
    let (abs_source, abs_target) = (abs_source.as_ref(), abs_target.as_ref());
    let rel_source = crate::util::relativize_path(abs_source, abs_target);
    if target_style == TargetStyle::Directory && rel_source.file_name().is_none() {
        if let Some(file_name) = abs_source.file_name() {
            force_symlink(rel_source, abs_target.join(file_name), TargetStyle::File)
        } else {
            Err(Error::new(
                LinkType::Symbolic,
                Clobber::FileOnly,
                rel_source,
                abs_target.to_owned(),
                target_style,
                ErrorCause::MissingFileName,
            ))
        }
    } else {
        force_symlink(rel_source, abs_target, target_style)
    }
}

fn create_symlink(target: &[u16], source: &[u16]) -> Result<(), runtime::Error> {
    if unsafe {
        CreateSymbolicLinkW(
            PWSTR(target.as_ptr() as _),
            PWSTR(source.as_ptr() as _),
            SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE,
        )
    } == BOOLEAN(0)
    {
        return Err(runtime::Error::from_win32().into());
    }
    Ok(())
}

fn delete_symlink(filename: &[u16]) -> Result<(), runtime::Error> {
    let handle = FileHandle(unsafe {
        CreateFileW(
            PWSTR(filename.as_ptr() as _),
            FILE_ACCESS_FLAGS(GENERIC_READ),
            FILE_SHARE_READ,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_DELETE_ON_CLOSE,
            HANDLE(0),
        )
    });
    if handle.is_invalid() {
        return Err(runtime::Error::from_win32());
    }
    Ok(())
}

fn is_symlink(filename: &[u16]) -> bool {
    let attr = unsafe { GetFileAttributesW(PWSTR(filename.as_ptr() as _)) };
    if attr & FILE_ATTRIBUTE_REPARSE_POINT.0 == 0 {
        return false;
    }
    let h_file = FileHandle(unsafe {
        CreateFileW(
            PWSTR(filename.as_ptr() as _),
            FILE_ACCESS_FLAGS(GENERIC_READ),
            FILE_SHARE_READ,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            HANDLE(0),
        )
    });
    if h_file.0 == INVALID_HANDLE_VALUE {
        return false;
    }
    let mut buffer: REPARSE_GUID_DATA_BUFFER = unsafe { std::mem::zeroed() };
    let mut bytes = 0u32;
    let result = unsafe {
        DeviceIoControl(
            h_file.0,
            FSCTL_GET_REPARSE_POINT,
            std::ptr::null(),
            0,
            &mut buffer as *mut _ as _,
            std::mem::size_of::<REPARSE_GUID_DATA_BUFFER>() as _,
            (&mut bytes) as _,
            std::ptr::null_mut(),
        )
    };
    if !result.as_bool() && unsafe { GetLastError() } != ERROR_MORE_DATA {
        return false;
    }
    return buffer.ReparseTag as i32 == IO_REPARSE_TAG_SYMLINK;
}

struct FileHandle(HANDLE);

impl FileHandle {
    fn is_invalid(&self) -> bool {
        self.0 == INVALID_HANDLE_VALUE
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        if !self.is_invalid() {
            unsafe { CloseHandle(self.0) };
        }
    }
}
