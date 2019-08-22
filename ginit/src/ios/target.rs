use crate::{
    config::Config,
    init::cargo::CargoTarget,
    opts::NoiseLevel,
    target::{Profile, TargetTrait},
    util,
};
use into_result::IntoResult as _;
use std::{collections::BTreeMap, path::Path, process::Command};

#[derive(Clone, Copy, Debug)]
pub struct Target<'a> {
    pub triple: &'a str,
    pub arch: &'a str,
}

impl<'a> TargetTrait<'a> for Target<'a> {
    fn all() -> &'a BTreeMap<&'a str, Self> {
        lazy_static::lazy_static! {
            pub static ref TARGETS: BTreeMap<&'static str, Target<'static>> = {
                let mut targets = BTreeMap::new();
                targets.insert("aarch64", Target {
                    triple: "aarch64-apple-ios",
                    arch: "arm64",
                });
                targets.insert("x86_64", Target {
                    triple: "x86_64-apple-ios",
                    arch: "x86_64",
                });
                targets
            };
        }
        &*TARGETS
    }

    fn triple(&'a self) -> &'a str {
        self.triple
    }

    fn arch(&'a self) -> &'a str {
        self.arch
    }
}

impl<'a> Target<'a> {
    // TODO: Make this cleaner
    pub fn macos() -> Self {
        Self {
            triple: "x86_64-apple-darwin",
            arch: "x86_64",
        }
    }

    pub fn generate_cargo_config(&self) -> CargoTarget {
        Default::default()
    }

    fn cargo(&'a self, config: &'a Config, subcommand: &'a str) -> util::CargoCommand<'a> {
        util::CargoCommand::new(subcommand)
            .with_package(Some(config.app_name()))
            .with_manifest_path(config.manifest_path())
            .with_target(Some(&self.triple))
            .with_features(Some("metal"))
    }

    pub fn check(&self, config: &Config, noise_level: NoiseLevel) {
        self.cargo(config, "check")
            .with_verbose(noise_level.is_verbose())
            .into_command()
            .status()
            .into_result()
            .expect("Failed to run `cargo check`");
    }

    pub fn compile_lib(&self, config: &Config, noise_level: NoiseLevel, profile: Profile) {
        // NOTE: it's up to Xcode to pass the verbose flag here, so even when
        // using our build/run commands it won't get passed.
        self.cargo(config, "build")
            .with_verbose(noise_level.is_verbose())
            .with_release(profile.is_release())
            .into_command()
            .status()
            .into_result()
            .expect("Failed to run `cargo build`");
    }

    pub fn build(config: &Config, profile: Profile) {
        let configuration = profile.as_str();
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

    fn archive(config: &Config, profile: Profile) {
        let configuration = profile.as_str();
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

    pub fn run(config: &Config, profile: Profile) {
        // TODO: These steps are run unconditionally, which is slooooooow
        Self::build(config, profile);
        Self::archive(config, profile);
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
