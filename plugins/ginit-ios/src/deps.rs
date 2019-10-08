use ginit_core::{opts::Clobbering, util};
use into_result::{command::CommandError, IntoResult as _};
use std::{
    fmt::{self, Display},
    path::Path,
    process::Command,
};

#[derive(Debug)]
pub enum Error {
    XcodeGenPresenceCheckFailed(CommandError),
    XcodeGenInstallFailed(CommandError),
    IosDeployPullFailed(CommandError),
    IosDeployCheckoutFailed(CommandError),
    IosDeployBuildFailed(CommandError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::XcodeGenPresenceCheckFailed(err) => {
                write!(f, "Failed to check for presence of `xcodegen`: {}", err)
            }
            Self::XcodeGenInstallFailed(err) => write!(f, "Failed to install `xcodegen`: {}", err),
            Self::IosDeployPullFailed(err) => {
                write!(f, "Failed to pull `ios-deploy` repo: {}", err)
            }
            Self::IosDeployCheckoutFailed(err) => {
                write!(f, "Failed to checkout `ios-deploy` repo: {}", err)
            }
            Self::IosDeployBuildFailed(err) => write!(f, "Failed to build `ios-deploy`: {}", err),
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

    // Installing `ios-deploy` normally involves npm, even though it doesn't
    // use JavaScript at all... so, let's build it manually!
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dest = root.join("ios-deploy");
    let ios_deploy_found = dest.join("build/Release/ios-deploy").exists();
    if !ios_deploy_found || clobbering.is_allowed() {
        if dest.exists() {
            util::git(&dest, &["pull", "--rebase", "origin", "master"])
                .map_err(Error::IosDeployPullFailed)?;
        } else {
            util::git(
                &root,
                &[
                    "clone",
                    "--depth",
                    "1",
                    "https://github.com/ios-control/ios-deploy",
                ],
            )
            .map_err(Error::IosDeployCheckoutFailed)?;
        }
        let project = dest.join("ios-deploy.xcodeproj");
        Command::new("xcodebuild")
            .arg("-project")
            .arg(&project)
            .status()
            .into_result()
            .map_err(Error::IosDeployBuildFailed)?;
    }
    Ok(())
}
