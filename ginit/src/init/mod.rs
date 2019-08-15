pub mod cargo;
pub mod config;
pub mod rust;

use self::cargo::CargoConfig;
use crate::{
    android,
    config::Config,
    ios,
    target::TargetTrait as _,
    util::{self, FriendlyContains},
};
use into_result::IntoResult as _;
use std::{path::Path, process::Command};

pub static STEPS: &'static [&'static str] = &[
    "deps",
    "toolchains",
    "cargo",
    "hello_world",
    "android",
    "ios",
];

#[derive(Clone, Copy, Debug, Default)]
pub struct Skip {
    pub deps: bool,
    pub toolchains: bool,
    pub cargo: bool,
    pub hello_world: bool,
    pub android: bool,
    pub ios: bool,
}

impl<'a, T> From<&'a [T]> for Skip
where
    &'a [T]: FriendlyContains<T>,
    str: PartialEq<T>,
{
    fn from(skip: &'a [T]) -> Self {
        Skip {
            deps: skip.friendly_contains("deps"),
            toolchains: skip.friendly_contains("toolchains"),
            cargo: skip.friendly_contains("cargo"),
            hello_world: skip.friendly_contains("hello_world"),
            android: skip.friendly_contains("android"),
            ios: skip.friendly_contains("ios"),
        }
    }
}

// TODO: Don't redo things if no changes need to be made
pub fn init(config: &Config, bike: &bicycle::Bicycle, force: bool, skip: impl Into<Skip>) {
    let skip = skip.into();
    if !skip.cargo {
        CargoConfig::generate().write(&config);
    }
    if !skip.hello_world {
        rust::hello_world(config, bike, force).unwrap();
    }
    if !skip.android {
        if !skip.toolchains {
            for target in android::target::Target::all().values() {
                target.rustup_add();
            }
        }
        android::project::create(config, bike).unwrap();
    }
    if !skip.ios {
        if !skip.deps {
            install_ios_deps(force);
        }
        if !skip.toolchains {
            for target in ios::target::Target::all().values() {
                target.rustup_add();
            }
        }
        ios::project::create(config, bike).unwrap();
    }
}

// TODO: We should probably also try to install `rust-xcode-plugin`
pub fn install_ios_deps(force: bool) {
    let xcodegen_found = util::command_present("xcodegen").expect("Failed to check for `xcodegen`");
    if !xcodegen_found || force {
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
    if !ios_deploy_found || force {
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
