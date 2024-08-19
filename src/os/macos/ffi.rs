#![allow(dead_code, non_snake_case, non_upper_case_globals)]

use core_foundation::{
    array::CFArrayRef, base::OSStatus, error::CFErrorRef, string::CFStringRef, url::CFURLRef,
};
use std::{os::raw::c_void, ptr};

// https://developer.apple.com/documentation/kernel/optionbits?language=objc
pub type OptionBits = u32;

// https://developer.apple.com/documentation/coreservices/lsrolesmask?language=objc
pub type LSRolesMask = OptionBits;

pub const kLSRolesNone: LSRolesMask = 0x00000001;
pub const kLSRolesViewer: LSRolesMask = 0x00000002;
pub const kLSRolesEditor: LSRolesMask = 0x00000004;
pub const kLSRolesShell: LSRolesMask = 0x00000008;
pub const kLSRolesAll: LSRolesMask = LSRolesMask::MAX;

// https://developer.apple.com/documentation/coreservices/lslaunchflags?language=objc
pub type LSLaunchFlags = OptionBits;

pub const kLSLaunchDefaults: LSLaunchFlags = 0x00000001;
pub const kLSLaunchAndPrint: LSLaunchFlags = 0x00000002;
pub const kLSLaunchAndDisplayErrors: LSLaunchFlags = 0x00000040;
pub const kLSLaunchDontAddToRecents: LSLaunchFlags = 0x00000100;
pub const kLSLaunchDontSwitch: LSLaunchFlags = 0x00000200;
pub const kLSLaunchAsync: LSLaunchFlags = 0x00010000;
pub const kLSLaunchNewInstance: LSLaunchFlags = 0x00080000;
pub const kLSLaunchAndHide: LSLaunchFlags = 0x00100000;
pub const kLSLaunchAndHideOthers: LSLaunchFlags = 0x00200000;

// https://developer.apple.com/documentation/coreservices/lslaunchurlspec?language=objc
// Field ordering: https://github.com/phracker/MacOSX-SDKs/blob/ef9fe35d5691b6dd383c8c46d867a499817a01b6/MacOSX10.3.9.sdk/System/Library/Frameworks/ApplicationServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Headers/LSOpen.h#L80
#[derive(Debug)]
#[repr(C)]
pub struct LSLaunchURLSpec {
    pub appURL: CFURLRef,
    pub itemURLs: CFArrayRef,
    // This should actually be `*const AEDesc`, but we don't personally need
    // this field, so we'll save ourselves the trouble of defining more structs.
    // Just, yknow, be sure to only ever set this to NULL!
    passThruParams: *const c_void,
    pub launchFlags: LSLaunchFlags,
    asyncRefCon: *mut c_void,
}

impl LSLaunchURLSpec {
    pub fn new(appURL: CFURLRef, itemURLs: CFArrayRef, launchFlags: LSLaunchFlags) -> Self {
        Self {
            appURL,
            asyncRefCon: ptr::null_mut(),
            itemURLs,
            launchFlags,
            passThruParams: ptr::null(),
        }
    }
}

// https://developer.apple.com/documentation/coreservices/launch_services?language=objc#1661359
pub const kLSAppInTrashErr: OSStatus = -10660;
pub const kLSUnknownErr: OSStatus = -10810;
pub const kLSNotAnApplicationErr: OSStatus = -10811;
pub const kLSNotInitializedErr: OSStatus = -10812;
pub const kLSDataUnavailableErr: OSStatus = -10813;
pub const kLSApplicationNotFoundErr: OSStatus = -10814;
pub const kLSUnknownTypeErr: OSStatus = -10815;
pub const kLSDataTooOldErr: OSStatus = -10816;
pub const kLSDataErr: OSStatus = -10817;
pub const kLSLaunchInProgressErr: OSStatus = -10818;
pub const kLSNotRegisteredErr: OSStatus = -10819;
pub const kLSAppDoesNotClaimTypeErr: OSStatus = -10820;
pub const kLSAppDoesNotSupportSchemeWarning: OSStatus = -10821;
pub const kLSServerCommunicationErr: OSStatus = -10822;
pub const kLSCannotSetInfoErr: OSStatus = -10823;
pub const kLSNoRegistrationInfoErr: OSStatus = -10824;
pub const kLSIncompatibleSystemVersionErr: OSStatus = -10825;
pub const kLSNoLaunchPermissionErr: OSStatus = -10826;
pub const kLSNoExecutableErr: OSStatus = -10827;
pub const kLSNoClassicEnvironmentErr: OSStatus = -10828;
pub const kLSMultipleSessionsNotSupportedErr: OSStatus = -10829;

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    // https://developer.apple.com/documentation/coreservices/1448824-lscopydefaultapplicationurlforur?language=objc
    pub fn LSCopyDefaultApplicationURLForURL(
        inURL: CFURLRef,
        inRoleMask: LSRolesMask,
        outError: *mut CFErrorRef,
    ) -> CFURLRef;

    // https://developer.apple.com/documentation/coreservices/1447734-lscopydefaultapplicationurlforco?language=objc
    pub fn LSCopyDefaultApplicationURLForContentType(
        inContentType: CFStringRef,
        inRoleMask: LSRolesMask,
        outError: *mut CFErrorRef,
    ) -> CFURLRef;

    // https://developer.apple.com/documentation/coreservices/1441986-lsopenfromurlspec?language=objc
    pub fn LSOpenFromURLSpec(
        inLaunchSpec: *const LSLaunchURLSpec,
        outLaunchedURL: *mut CFURLRef,
    ) -> OSStatus;
}
