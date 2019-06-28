use crate::{
    init::CargoTarget,
    target::{get_possible_values, TargetTrait},
    util::{self, IntoResult},
    CONFIG,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

lazy_static::lazy_static! {
    pub static ref POSSIBLE_TARGETS: Vec<&'static str> = {
        get_possible_values::<Target>()
    };

    // TODO: Make this cleaner
    pub static ref MACOS: Target = Target {
        triple: "x86_64-apple-darwin".to_string(),
        arch: "x86_64".to_string(),
    };

    static ref SCHEME: String = format!("{}_iOS", CONFIG.app_name());

    static ref WORKSPACE_PATH: PathBuf = CONFIG.ios.project_root()
        .join(format!("{}.xcodeproj/project.xcworkspace/", CONFIG.app_name()));
    static ref EXPORT_PATH: PathBuf = CONFIG.ios.project_root()
        .join("build");
    static ref EXPORT_PLIST_PATH: PathBuf = CONFIG.ios.project_root()
        .join("ExportOptions.plist");
    static ref IPA_PATH: PathBuf = EXPORT_PATH
        .join(format!("{}.ipa", &*SCHEME));
    static ref APP_PATH: PathBuf = EXPORT_PATH
        .join(format!("Payload/{}.app", CONFIG.app_name()));
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Target {
    pub triple: String,
    pub arch: String,
}

impl TargetTrait for Target {
    fn all() -> &'static BTreeMap<String, Self> {
        &CONFIG.ios.targets
    }
    fn triple(&self) -> &str {
        &self.triple
    }
    fn arch(&self) -> &str {
        &self.arch
    }
}

impl Target {
    // NOTE: We still can't set ENABLE_BITCODE to true, since stdlib components
    // aren't built with bitcode: https://github.com/rust-lang/rust/issues/35968
    pub fn get_cargo_config(&self) -> CargoTarget {
        CargoTarget {
            ar: None,
            linker: None,
            rustflags: vec![],
        }
    }

    fn cargo<'a>(&'a self, subcommand: &'a str) -> util::CargoCommand<'a> {
        util::CargoCommand::new(subcommand)
            .with_package(Some(CONFIG.app_name()))
            .with_manifest_path(CONFIG.manifest_path())
            .with_target(Some(&self.triple))
            .with_features(Some("metal"))
    }

    pub fn check(&self, verbose: bool) {
        self.cargo("check")
            .with_verbose(verbose)
            .into_command()
            .status()
            .into_result()
            .expect("Failed to run `cargo check`");
    }

    pub fn compile_lib(&self, verbose: bool, release: bool) {
        // NOTE: it's up to Xcode to pass the verbose flag here, so even when
        // using our build/run commands it won't get passed.
        self.cargo("build")
            .with_verbose(verbose)
            .with_release(release)
            .into_command()
            .status()
            .into_result()
            .expect("Failed to run `cargo build`");
    }

    fn configuration(release: bool) -> &'static str {
        if release {
            "release"
        } else {
            "debug"
        }
    }

    pub fn build(release: bool) {
        let config = Self::configuration(release);
        Command::new("xcodebuild")
            .args(&["-scheme", &*SCHEME])
            .arg("-workspace")
            .arg(&*WORKSPACE_PATH)
            .args(&["-configuration", config])
            .arg("build")
            .status()
            .into_result()
            .expect("Failed to run `xcodebuild`");
    }

    fn archive(release: bool) {
        let config = Self::configuration(release);
        let archive_path = EXPORT_PATH.join(&*SCHEME);
        Command::new("xcodebuild")
            .args(&["-scheme", &*SCHEME])
            .arg("-workspace")
            .arg(&*WORKSPACE_PATH)
            .args(&["-sdk", "iphoneos"])
            .args(&["-configuration", config])
            .arg("archive")
            .arg("-archivePath")
            .arg(&archive_path)
            .status()
            .into_result()
            .expect("Failed to run `xcodebuild`");
        // Super fun discrepancy in expectation of `-archivePath` value
        let archive_path = EXPORT_PATH.join(&format!("{}.xcarchive", &*SCHEME));
        Command::new("xcodebuild")
            .arg("-exportArchive")
            .arg("-archivePath")
            .arg(&archive_path)
            .arg("-exportOptionsPlist")
            .arg(&*EXPORT_PLIST_PATH)
            .arg("-exportPath")
            .arg(&*EXPORT_PATH)
            .status()
            .into_result()
            .expect("Failed to run `xcodebuild`");
    }

    fn ios_deploy() -> Command {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("ios-deploy/build/Release/ios-deploy");
        if !path.exists() {
            panic!(
                "`ios-deploy` not found. Please run `cargo {} install-deps` and try again.",
                crate::NAME,
            );
        }
        Command::new(path)
    }

    pub fn run(release: bool) {
        // TODO: These steps are run unconditionally, which is slooooooow
        Self::build(release);
        Self::archive(release);
        Command::new("unzip")
            .arg("-o") // -o = always overwrite
            .arg(&*IPA_PATH)
            .arg("-d")
            .arg(&*EXPORT_PATH)
            .status()
            .into_result()
            .expect("Failed to run `unzip`");
        // This dies if the device is locked, and gives you no time to react to
        // that. `ios-deploy --detect` can apparently be used to check in
        // advance, giving us an opportunity to promt. Though, it's much more
        // relaxing to just turn off auto-lock under Display & Brightness.
        Self::ios_deploy()
            .arg("--debug")
            .arg("--bundle")
            .arg(&*APP_PATH)
            // This tool can apparently install over wifi, but not debug over
            // wifi... so if your device is connected over wifi (even if it's
            // wired as well) and we're using the `--debug` flag, then
            // launching will fail unless we also specify the `--no-wifi` flag
            // to keep it from trying that.
            .arg("--no-wifi")
            .status()
            .into_result()
            .expect("Failed to run `ios-deploy`");
    }
}
