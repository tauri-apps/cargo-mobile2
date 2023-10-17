use thiserror::Error;
use windows::Win32::System::{
    SystemInformation::{
        VerSetConditionMask, VerifyVersionInfoW, OSVERSIONINFOEXW, VER_BUILDNUMBER,
        VER_MAJORVERSION, VER_MINORVERSION, VER_PRODUCT_TYPE, VER_SERVICEPACKMAJOR,
    },
    SystemServices::{VER_EQUAL, VER_GREATER_EQUAL},
};

use crate::os::Info;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to find Version")]
    VersionMissing,
}

// (name, major_version, minor_version, service_pack, product_type, build_number)
const VERSION_LIST: &[(&str, u32, u32, u16, u8, Option<u32>)] = &[
    ("Windows 11", 10, 0, 0, 1, Some(22000)),
    ("Windows 10", 10, 0, 0, 1, None),
    ("Windows Server 2019", 10, 0, 0, 3, Some(17623)),
    ("Windows Server 2016", 10, 0, 0, 3, None),
    ("Windows 8.1", 6, 3, 0, 1, None),
    ("Windows Server 2012 R2", 6, 3, 0, 3, None),
    ("Windows 8", 6, 2, 0, 1, None),
    ("Windows Server 2012", 6, 2, 0, 3, None),
    ("Windows 7 Service Pack 1", 6, 1, 1, 1, None),
    ("Windows 7", 6, 1, 0, 1, None),
    ("Windows Server 2008 R2 Service Pack 1", 6, 1, 1, 3, None),
    ("Windows Server 2008 R2", 6, 1, 0, 3, None),
    ("Windows Server 2008", 6, 0, 0, 3, None),
    ("Windows Vista Service Pack 2", 6, 0, 2, 1, None),
    ("Windows Vista Service Pack 1", 6, 0, 1, 1, None),
    ("Windows Vista", 6, 0, 0, 1, None),
    // How to identify Windows Server 2003 R2 is unknown.
    ("Windows Server 2003 Service Pack 2", 5, 2, 2, 3, None),
    ("Windows Server 2003 Service Pack 1", 5, 2, 1, 3, None),
    ("Windows Server 2003", 5, 2, 0, 3, None),
    ("Windows XP 64-Bit Edition", 5, 2, 0, 1, None),
    ("Windows XP Service Pack 3", 5, 1, 3, 1, None),
    ("Windows XP Service Pack 2", 5, 1, 2, 1, None),
    ("Windows XP Service Pack 1", 5, 1, 1, 1, None),
    ("Windows XP", 5, 1, 0, 1, None),
    ("Windows 2000", 5, 0, 0, 1, None),
    // Older versions are omitted.
];

pub fn check() -> Result<Info, Error> {
    let mut osvi = OSVERSIONINFOEXW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOEXW>() as _,
        dwMajorVersion: 0,
        dwMinorVersion: 0,
        dwBuildNumber: 0,
        dwPlatformId: 0,
        szCSDVersion: [0; 128],
        wServicePackMajor: 0,
        wServicePackMinor: 0,
        wSuiteMask: 0,
        wProductType: 0,
        wReserved: 0,
    };
    let condition_mask =
        unsafe { VerSetConditionMask(0, VER_MAJORVERSION, VER_GREATER_EQUAL as u8) };
    let condition_mask =
        unsafe { VerSetConditionMask(condition_mask, VER_MINORVERSION, VER_GREATER_EQUAL as u8) };
    let condition_mask = unsafe {
        VerSetConditionMask(
            condition_mask,
            VER_SERVICEPACKMAJOR,
            VER_GREATER_EQUAL as u8,
        )
    };
    let condition_mask =
        unsafe { VerSetConditionMask(condition_mask, VER_PRODUCT_TYPE, VER_EQUAL as u8) };
    let type_mask = VER_MAJORVERSION | VER_MINORVERSION | VER_SERVICEPACKMAJOR | VER_PRODUCT_TYPE;

    for &(name, major, minor, service_pack, product_type, build_number) in VERSION_LIST {
        osvi.dwMajorVersion = major;
        osvi.dwMinorVersion = minor;
        osvi.wServicePackMajor = service_pack;
        osvi.wProductType = product_type;
        let (condition_mask, type_mask) = if let Some(build_number) = build_number {
            let condition_mask = unsafe {
                VerSetConditionMask(condition_mask, VER_BUILDNUMBER, VER_GREATER_EQUAL as u8)
            };
            let type_mask = type_mask | VER_BUILDNUMBER;
            osvi.dwBuildNumber = build_number;
            (condition_mask, type_mask)
        } else {
            (condition_mask, type_mask)
        };
        if unsafe { VerifyVersionInfoW(&mut osvi as *mut _, type_mask, condition_mask) }.is_ok() {
            return Ok(Info {
                name: name.to_string(),
                version: format!("{}.{}", major, minor),
            });
        };
    }

    Err(Error::VersionMissing)
}
