mod cargo;
mod config;
mod rust;

pub use self::cargo::CargoTarget;
use self::{cargo::CargoConfig, config::interactive_config_gen};
use crate::{android, Config, ios, util::{self, FriendlyContains, IntoResult}};
use std::{fs, path::Path, process::Command};

pub static STEPS: &'static [&'static str] = &[
    "cargo",
    "android",
    "ios",
];

#[derive(Debug)]
struct Skip {
    pub cargo: bool,
    pub hello_world: bool,
    pub android: bool,
    pub ios: bool,
}

impl From<Vec<String>> for Skip {
    fn from(skip: Vec<String>) -> Self {
        Skip {
            cargo: skip.friendly_contains("cargo"),
            hello_world: skip.friendly_contains("hello_world"),
            android: skip.friendly_contains("android"),
            ios: skip.friendly_contains("ios"),
        }
    }
}

// TODO: Don't redo things if no changes need to be made
pub fn init(force: bool, skip: Vec<String>) {
    if !Config::exists() {
        interactive_config_gen();
        Config::recheck_path();
    }
    let skip = Skip::from(skip);
    if !skip.cargo {
        CargoConfig::generate().write();
    }
    if !skip.hello_world {
        rust::hello_world(force).unwrap();
    }
    if !skip.android {
        android::project::create().unwrap();
    }
    if !skip.ios {
        ios::project::create().unwrap();
    }
}

// TODO: We should probably also try to install `rust-xcode-plugin`
pub fn install_deps() {
    Command::new("brew")
        .args(&["install", "xcodegen"])
        .status()
        .into_result()
        .expect("Failed to install `xcodegen`");

    // Installing `ios-deploy` normally involves npm, even though it doesn't
    // use JavaScript at all... so, let's build it manually!
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dest = root.join("ios-deploy");
    if dest.exists() {
        // see `fs::remove_dir_all` below
        util::git(&dest, &["checkout", "HEAD", "--", "ios-deploy.xcodeproj"])
            .expect("Failed to reset `ios-deploy` repo");
        util::git(&dest, &["pull", "--rebase", "origin", "master"])
            .expect("Failed to pull `ios-deploy` repo");
    } else {
        util::git(&root, &[
            "clone", "--depth", "1",
            "https://github.com/ios-control/ios-deploy",
        ]).expect("Failed to checkout `ios-deploy` repo");
    }
    let project = dest.join("ios-deploy.xcodeproj");
    Command::new("xcodebuild")
        .arg("-project").arg(&project)
        .status()
        .into_result()
        .expect("Failed to build `ios-deploy`");
    // Since we're currently putting our cargo tool in our rust folder
    // (which is symlinked into our Xcode project), this project ends up
    // getting detected and adding targets in Xcode... so, we can just
    // delete it for now.
    fs::remove_dir_all(project)
        .expect("Failed to delete `ios-deploy.xcodeproj`");
}
