use ginit_core::{
    exports::into_result::{command::CommandError, IntoResult as _},
    opts::Clobbering,
    util,
};
use std::{
    fmt::{self, Display},
    process::Command,
};

#[derive(Debug)]
pub enum Error {
    XcodeGenPresenceCheckFailed(CommandError),
    XcodeGenInstallFailed(CommandError),
    IosDeployPresenceCheckFailed(CommandError),
    IosDeployInstallFailed(CommandError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::XcodeGenPresenceCheckFailed(err) => {
                write!(f, "Failed to check for presence of `xcodegen`: {}", err)
            }
            Self::XcodeGenInstallFailed(err) => write!(f, "Failed to install `xcodegen`: {}", err),
            Self::IosDeployPresenceCheckFailed(err) => {
                write!(f, "Failed to check for presence of `ios-deploy`: {}", err)
            }
            Self::IosDeployInstallFailed(err) => {
                write!(f, "Failed to install `ios-deploy`: {}", err)
            }
        }
    }
}

// TODO: We should probably also try to install `rust-xcode-plugin`
pub fn install(clobbering: Clobbering) -> Result<(), Error> {
    let xcodegen_found =
        util::command_present("xcodegen").map_err(Error::XcodeGenPresenceCheckFailed)?;
    if !xcodegen_found || clobbering.is_allowed() {
        Command::new("brew")
            // reinstall works even if it's not installed yet,
            // and will upgrade if it's already installed!
            .args(&["reinstall", "xcodegen"])
            .status()
            .into_result()
            .map_err(Error::XcodeGenInstallFailed)?;
    }
    let ios_deploy_found =
        util::command_present("ios-deploy").map_err(Error::IosDeployPresenceCheckFailed)?;
    if !ios_deploy_found || clobbering.is_allowed() {
        Command::new("brew")
            .args(&["reinstall", "ios-deploy"])
            .status()
            .into_result()
            .map_err(Error::IosDeployInstallFailed)?;
    }
    Ok(())
}
