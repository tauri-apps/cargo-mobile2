use ginit_core::{exports::bossy, opts::Clobbering, util};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    XcodeGenPresenceCheckFailed(bossy::Error),
    XcodeGenInstallFailed(bossy::Error),
    IosDeployPresenceCheckFailed(bossy::Error),
    IosDeployInstallFailed(bossy::Error),
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
        bossy::Command::impure("brew")
            // reinstall works even if it's not installed yet,
            // and will upgrade if it's already installed!
            .with_args(&["reinstall", "xcodegen"])
            .run_and_wait()
            .map_err(Error::XcodeGenInstallFailed)?;
    }
    let ios_deploy_found =
        util::command_present("ios-deploy").map_err(Error::IosDeployPresenceCheckFailed)?;
    if !ios_deploy_found || clobbering.is_allowed() {
        bossy::Command::impure("brew")
            .with_args(&["reinstall", "ios-deploy"])
            .run_and_wait()
            .map_err(Error::IosDeployInstallFailed)?;
    }
    Ok(())
}
