use crate::{
    config::Config,
    init::cargo::CargoTarget,
    target::TargetTrait,
    util::{self, IntoResult},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path, process::Command};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Target {
    pub triple: String,
    pub arch: String,
}

impl TargetTrait for Target {
    fn all(config: &Config) -> &BTreeMap<String, Self> {
        config.ios().targets()
    }

    fn triple(&self) -> &str {
        &self.triple
    }

    fn arch(&self) -> &str {
        &self.arch
    }
}

impl Target {
    // TODO: Make this cleaner
    pub fn macos() -> Self {
        Self {
            triple: "x86_64-apple-darwin".to_string(),
            arch: "x86_64".to_string(),
        }
    }

    pub fn generate_cargo_config(&self) -> CargoTarget {
        Default::default()
    }

    fn cargo<'a>(&'a self, config: &'a Config, subcommand: &'a str) -> util::CargoCommand<'a> {
        util::CargoCommand::new(subcommand)
            .with_package(Some(config.app_name()))
            .with_manifest_path(config.manifest_path())
            .with_target(Some(&self.triple))
            .with_features(Some("metal"))
    }

    pub fn check(&self, config: &Config, verbose: bool) {
        self.cargo(config, "check")
            .with_verbose(verbose)
            .into_command()
            .status()
            .into_result()
            .expect("Failed to run `cargo check`");
    }

    pub fn compile_lib(&self, config: &Config, verbose: bool, release: bool) {
        // NOTE: it's up to Xcode to pass the verbose flag here, so even when
        // using our build/run commands it won't get passed.
        self.cargo(config, "build")
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

    pub fn build(config: &Config, release: bool) {
        let configuration = Self::configuration(release);
        Command::new("xcodebuild")
            .args(&["-scheme", &config.ios().scheme()])
            .arg("-workspace")
            .arg(&config.ios().workspace_path())
            .args(&["-configuration", configuration])
            .arg("build")
            .status()
            .into_result()
            .expect("Failed to run `xcodebuild`");
    }

    fn archive(config: &Config, release: bool) {
        let configuration = Self::configuration(release);
        let archive_path = config.ios().export_path().join(&config.ios().scheme());
        Command::new("xcodebuild")
            .args(&["-scheme", &config.ios().scheme()])
            .arg("-workspace")
            .arg(&config.ios().workspace_path())
            .args(&["-sdk", "iphoneos"])
            .args(&["-configuration", configuration])
            .arg("archive")
            .arg("-archivePath")
            .arg(&archive_path)
            .status()
            .into_result()
            .expect("Failed to run `xcodebuild`");
        // Super fun discrepancy in expectation of `-archivePath` value
        let archive_path = config
            .ios()
            .export_path()
            .join(&format!("{}.xcarchive", config.ios().scheme()));
        Command::new("xcodebuild")
            .arg("-exportArchive")
            .arg("-archivePath")
            .arg(&archive_path)
            .arg("-exportOptionsPlist")
            .arg(&config.ios().export_plist_path())
            .arg("-exportPath")
            .arg(&config.ios().export_path())
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

    pub fn run(config: &Config, release: bool) {
        // TODO: These steps are run unconditionally, which is slooooooow
        Self::build(config, release);
        Self::archive(config, release);
        Command::new("unzip")
            .arg("-o") // -o = always overwrite
            .arg(&config.ios().ipa_path())
            .arg("-d")
            .arg(&config.ios().export_path())
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
            .arg(&config.ios().app_path())
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
