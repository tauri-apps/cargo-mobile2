use super::{
    system_profile::{self, DeveloperTools},
    xcode_plugin,
};
use crate::{
    opts,
    util::{
        self,
        cli::{Report, TextWrapper},
    },
};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    XcodeGenPresenceCheckFailed(bossy::Error),
    XcodeGenInstallFailed(bossy::Error),
    IosDeployPresenceCheckFailed(bossy::Error),
    IosDeployInstallFailed(bossy::Error),
    VersionLookupFailed(system_profile::Error),
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
            Self::VersionLookupFailed(err) => write!(f, "{}", err),
        }
    }
}

pub fn install(
    wrapper: &TextWrapper,
    skip_dev_tools: opts::SkipDevTools,
    reinstall_deps: opts::ReinstallDeps,
) -> Result<(), Error> {
    let xcodegen_found =
        util::command_present("xcodegen").map_err(Error::XcodeGenPresenceCheckFailed)?;
    if !xcodegen_found || reinstall_deps.yes() {
        bossy::Command::impure("brew")
            // reinstall works even if it's not installed yet,
            // and will upgrade if it's already installed!
            .with_args(&["reinstall", "xcodegen"])
            .run_and_wait()
            .map_err(Error::XcodeGenInstallFailed)?;
    }
    let ios_deploy_found =
        util::command_present("ios-deploy").map_err(Error::IosDeployPresenceCheckFailed)?;
    if !ios_deploy_found || reinstall_deps.yes() {
        bossy::Command::impure("brew")
            .with_args(&["reinstall", "ios-deploy"])
            .run_and_wait()
            .map_err(Error::IosDeployInstallFailed)?;
    }
    // we definitely don't want to install this on CI...
    if skip_dev_tools.no() {
        let tool_info = DeveloperTools::new().map_err(Error::VersionLookupFailed)?;
        let result = xcode_plugin::install(wrapper, reinstall_deps, tool_info.version);
        if let Err(err) = result {
            // philosophy: never be so sturbborn as to prevent use / progress
            Report::action_request(
                "Failed to install Rust Xcode plugin; this component is optional, so init will continue anyway, but Xcode debugging won't work until this is resolved!",
                err,
            )
            .print(wrapper);
        }
    }
    Ok(())
}
