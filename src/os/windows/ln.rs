use super::Windows::Win32::{Foundation::PWSTR, Storage::FileSystem::CreateHardLinkW};
use crate::util::ln::{Clobber, Error, ErrorCause, LinkType, TargetStyle};
use std::{os::windows::ffi::OsStrExt, path::Path};

pub fn force_hard_link_or_copy_file(
    src: impl AsRef<Path>,
    dest: impl AsRef<Path>,
) -> Result<(), Error> {
    let src = src.as_ref();
    let dest = dest.as_ref();
    let mut src_buf = src.as_os_str().encode_wide().collect::<Vec<_>>();
    let mut dest_buf = dest.as_os_str().encode_wide().collect::<Vec<_>>();
    src_buf.push(0);
    dest_buf.push(0);
    if dest.is_file() {
        std::fs::remove_file(dest).map_err(|err| {
            Error::new(
                LinkType::Hard,
                Clobber::FileOnly,
                src.to_owned(),
                dest.to_owned(),
                TargetStyle::File,
                ErrorCause::IOError(err),
            )
        })?;
    }
    if !unsafe {
        CreateHardLinkW(
            PWSTR(dest_buf.as_mut_ptr()),
            PWSTR(src_buf.as_mut_ptr()),
            std::ptr::null_mut(),
        )
    }
    .as_bool()
    {
        // If the drive is different between src and dest, the creation of the hard link will fail.
        // In that case, fall back to the simple copy.
        std::fs::copy(src, dest).map_err(|err| {
            Error::new(
                LinkType::Hard,
                Clobber::FileOnly,
                src.to_owned(),
                dest.to_owned(),
                TargetStyle::File,
                ErrorCause::IOError(err),
            )
        })?;
    }
    Ok(())
}
