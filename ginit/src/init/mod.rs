pub mod cargo;
pub mod config_gen;
pub mod migrate;
pub mod rust;
pub mod steps;

use self::{cargo::CargoConfig, steps::Steps};
use crate::{
    android,
    config::Config,
    ios,
    opts::Clobbering,
    target::TargetTrait as _,
    util::{
        self,
        prompt::{self, YesOrNo},
    },
};
use into_result::{command::CommandError, IntoResult as _};
use std::{fmt, io, path::Path, process::Command};

#[derive(Debug)]
pub enum Error {
    MigrationPromptFailed(io::Error),
    MigrationFailed(migrate::Error),
    CargoConfigGenFailed(cargo::GenError),
    CargoConfigWriteFailed(cargo::WriteError),
    HelloWorldGenFailed(rust::Error),
    AndroidGenFailed(android::project::Error),
    IosDepsFailed(IosDepsError),
    IosGenFailed(ios::project::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MigrationPromptFailed(err) => {
                write!(f, "Failed to prompt for migration: {}", err)
            }
            Error::MigrationFailed(err) => write!(
                f,
                "Failed to migrate project - project state is now undefined! 💀: {}",
                err
            ),
            Error::CargoConfigGenFailed(err) => {
                write!(f, "Failed to generate \".cargo/config\": {}", err)
            }
            Error::CargoConfigWriteFailed(err) => {
                write!(f, "Failed to write \".cargo/config\": {}", err)
            }
            Error::HelloWorldGenFailed(err) => {
                write!(f, "Failed to generate hello world project: {}", err)
            }
            Error::AndroidGenFailed(err) => {
                write!(f, "Failed to generate Android project: {}", err)
            }
            Error::IosDepsFailed(err) => write!(f, "Failed to install iOS dependencies: {}", err),
            Error::IosGenFailed(err) => write!(f, "Failed to generate iOS project: {}", err),
        }
    }
}

// TODO: Don't redo things if no changes need to be made
pub fn init(
    config: &Config,
    bike: &bicycle::Bicycle,
    clobbering: Clobbering,
    only: Option<impl Into<Steps>>,
    skip: Option<impl Into<Steps>>,
) -> Result<(), Error> {
    if let Some(proj) = migrate::LegacyProject::heuristic_detect(config) {
        println!(
            r#"
It looks like you're using the old project structure, which is now unsupported.
The new project structure is super sleek, and ginit can migrate your project
automatically! However, this can potentially fail. Be sure you have a backup of
your project in case things explode. You've been warned! 💀
        "#
        );
        let response = prompt::yes_no(
            "I have a backup, and I'm ready to migrate",
            Some(YesOrNo::No),
        )
        .map_err(Error::MigrationPromptFailed)?;
        match response {
            Some(YesOrNo::Yes) => {
                proj.migrate(config).map_err(Error::MigrationFailed)?;
                println!("Migration successful! 🎉\n");
            }
            Some(YesOrNo::No) => {
                println!("Maybe next time. Buh-bye!");
                return Ok(());
            }
            None => {
                println!("That was neither a Y nor an N! You're pretty silly.");
                return Ok(());
            }
        }
    }
    let steps = {
        let only = only.map(Into::into).unwrap_or_else(|| Steps::all());
        let skip = skip.map(Into::into).unwrap_or_else(|| Steps::empty());
        only & !skip
    };
    if steps.contains(Steps::CARGO) {
        CargoConfig::generate(config, &steps)
            .map_err(Error::CargoConfigGenFailed)?
            .write(&config)
            .map_err(Error::CargoConfigWriteFailed)?;
    }
    if steps.contains(Steps::HELLO_WORLD) {
        rust::hello_world(config, bike, clobbering).map_err(Error::HelloWorldGenFailed)?;
    }
    if steps.contains(Steps::ANDROID) {
        if steps.contains(Steps::TOOLCHAINS) {
            for target in android::target::Target::all().values() {
                target.rustup_add();
            }
        }
        android::project::create(config, bike).map_err(Error::AndroidGenFailed)?;
    }
    if steps.contains(Steps::IOS) {
        if steps.contains(Steps::DEPS) {
            install_ios_deps(clobbering).map_err(Error::IosDepsFailed)?;
        }
        if steps.contains(Steps::TOOLCHAINS) {
            for target in ios::target::Target::all().values() {
                target.rustup_add();
            }
        }
        ios::project::create(config, bike).map_err(Error::IosGenFailed)?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum IosDepsError {
    XcodeGenPresenceCheckFailed(CommandError),
    XcodeGenInstallFailed(CommandError),
    IosDeployPullFailed(CommandError),
    IosDeployCheckoutFailed(CommandError),
    IosDeployBuildFailed(CommandError),
}

impl fmt::Display for IosDepsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IosDepsError::XcodeGenPresenceCheckFailed(err) => {
                write!(f, "Failed to check for presence of `xcodegen`: {}", err)
            }
            IosDepsError::XcodeGenInstallFailed(err) => {
                write!(f, "Failed to install `xcodegen`: {}", err)
            }
            IosDepsError::IosDeployPullFailed(err) => {
                write!(f, "Failed to pull `ios-deploy` repo: {}", err)
            }
            IosDepsError::IosDeployCheckoutFailed(err) => {
                write!(f, "Failed to checkout `ios-deploy` repo: {}", err)
            }
            IosDepsError::IosDeployBuildFailed(err) => {
                write!(f, "Failed to build `ios-deploy`: {}", err)
            }
        }
    }
}

// TODO: We should probably also try to install `rust-xcode-plugin`
pub fn install_ios_deps(clobbering: Clobbering) -> Result<(), IosDepsError> {
    let xcodegen_found =
        util::command_present("xcodegen").map_err(IosDepsError::XcodeGenPresenceCheckFailed)?;
    if !xcodegen_found || clobbering.is_allowed() {
        Command::new("brew")
            // reinstall works even if it's not installed yet,
            // and will upgrade if it's already installed!
            .args(&["reinstall", "xcodegen"])
            .status()
            .into_result()
            .map_err(IosDepsError::XcodeGenInstallFailed)?;
    }

    // Installing `ios-deploy` normally involves npm, even though it doesn't
    // use JavaScript at all... so, let's build it manually!
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dest = root.join("ios-deploy");
    let ios_deploy_found = dest.join("build/Release/ios-deploy").exists();
    if !ios_deploy_found || clobbering.is_allowed() {
        if dest.exists() {
            util::git(&dest, &["pull", "--rebase", "origin", "master"])
                .map_err(IosDepsError::IosDeployPullFailed)?;
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
            .map_err(IosDepsError::IosDeployCheckoutFailed)?;
        }
        let project = dest.join("ios-deploy.xcodeproj");
        Command::new("xcodebuild")
            .arg("-project")
            .arg(&project)
            .status()
            .into_result()
            .map_err(IosDepsError::IosDeployBuildFailed)?;
    }
    Ok(())
}
