pub mod cargo;
pub mod config_gen;
pub mod rust;
pub mod steps;

use self::{cargo::CargoConfig, steps::Steps};
use crate::{android, config::Config, ios, opts::Clobbering, target::TargetTrait as _, util};
use into_result::IntoResult as _;
use std::{path::Path, process::Command};

// TODO: Don't redo things if no changes need to be made
pub fn init(
    config: &Config,
    bike: &bicycle::Bicycle,
    clobbering: Clobbering,
    only: Option<impl Into<Steps>>,
    skip: Option<impl Into<Steps>>,
) {
    let steps = {
        let only = only.map(Into::into).unwrap_or_else(|| Steps::all());
        let skip = skip.map(Into::into).unwrap_or_else(|| Steps::empty());
        only & !skip
    };
    if steps.contains(Steps::CARGO) {
        CargoConfig::generate().write(&config);
    }
    if steps.contains(Steps::HELLO_WORLD) {
        rust::hello_world(config, bike, clobbering).unwrap();
    }
    if steps.contains(Steps::ANDROID) {
        if steps.contains(Steps::TOOLCHAINS) {
            for target in android::target::Target::all().values() {
                target.rustup_add();
            }
        }
        android::project::create(config, bike).unwrap();
    }
    if steps.contains(Steps::IOS) {
        if steps.contains(Steps::DEPS) {
            install_ios_deps(clobbering);
        }
        if steps.contains(Steps::TOOLCHAINS) {
            for target in ios::target::Target::all().values() {
                target.rustup_add();
            }
        }
        ios::project::create(config, bike).unwrap();
    }
}

// TODO: We should probably also try to install `rust-xcode-plugin`
pub fn install_ios_deps(clobbering: Clobbering) {
    let xcodegen_found = util::command_present("xcodegen").expect("Failed to check for `xcodegen`");
    if !xcodegen_found || clobbering.is_allowed() {
        Command::new("brew")
            // reinstall works even if it's not installed yet,
            // and will upgrade if it's already installed!
            .args(&["reinstall", "xcodegen"])
            .status()
            .into_result()
            .expect("Failed to install `xcodegen`");
    }

    // Installing `ios-deploy` normally involves npm, even though it doesn't
    // use JavaScript at all... so, let's build it manually!
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dest = root.join("ios-deploy");
    let ios_deploy_found = dest.join("build/Release/ios-deploy").exists();
    if !ios_deploy_found || clobbering.is_allowed() {
        if dest.exists() {
            util::git(&dest, &["pull", "--rebase", "origin", "master"])
                .expect("Failed to pull `ios-deploy` repo");
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
            .expect("Failed to checkout `ios-deploy` repo");
        }
        let project = dest.join("ios-deploy.xcodeproj");
        Command::new("xcodebuild")
            .arg("-project")
            .arg(&project)
            .status()
            .into_result()
            .expect("Failed to build `ios-deploy`");
    }
}
