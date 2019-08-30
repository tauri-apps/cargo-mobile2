use crate::{
    config::Config,
    env::Env,
    init::cargo::CargoTarget,
    ios::system_profile::DeveloperTools,
    opts::NoiseLevel,
    target::{Profile, TargetTrait},
    util::{self, pure_command::PureCommand},
};
use into_result::IntoResult as _;
use std::{collections::BTreeMap, path::Path, process::Command};

fn ios_deploy(env: &Env) -> Command {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ios-deploy/build/Release/ios-deploy");
    if !path.exists() {
        panic!(
            "`ios-deploy` not found. Please run `cargo {} install-deps` and try again.",
            crate::NAME,
        );
    }
    PureCommand::new(path, env)
}

#[derive(Clone, Copy, Debug)]
pub struct Target<'a> {
    pub triple: &'a str,
    pub arch: &'a str,
    min_xcode_version: Option<((u32, u32), &'static str)>,
}

impl<'a> TargetTrait<'a> for Target<'a> {
    const DEFAULT_KEY: &'static str = "aarch64";

    fn all() -> &'a BTreeMap<&'a str, Self> {
        lazy_static::lazy_static! {
            pub static ref TARGETS: BTreeMap<&'static str, Target<'static>> = {
                let mut targets = BTreeMap::new();
                targets.insert("aarch64", Target {
                    triple: "aarch64-apple-ios",
                    arch: "arm64",
                    min_xcode_version: None,
                });
                targets.insert("x86_64", Target {
                    triple: "x86_64-apple-ios",
                    arch: "x86_64",
                    // Simulator only supports Metal as of Xcode 11.0:
                    // https://developer.apple.com/documentation/metal/developing_metal_apps_that_run_in_simulator?language=objc
                    // While this doesn't matter if you aren't using Metal,
                    // it should be fine to be opinionated about this given
                    // OpenGL's deprecation.
                    min_xcode_version: Some(((11, 0), "iOS Simulator doesn't support Metal until")),
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
            min_xcode_version: None,
        }
    }

    pub fn generate_cargo_config(&self) -> CargoTarget {
        Default::default()
    }

    fn min_xcode_version_satisfied(&self) -> Result<(), String> {
        self.min_xcode_version
            .map(|(min_version, msg)| {
                let tool_info = DeveloperTools::new().expect("Failed to get developer tool info");
                let installed_version = tool_info.version;
                if installed_version >= min_version {
                    Ok(())
                } else {
                    Err(format!(
                        "{} Xcode {}.{}; you have Xcode {}.{}",
                        msg, min_version.0, min_version.1, installed_version.0, installed_version.1
                    ))
                }
            })
            .unwrap_or_else(|| Ok(()))
    }

    fn cargo(&'a self, config: &'a Config, subcommand: &'a str) -> util::CargoCommand<'a> {
        if let Err(msg) = self.min_xcode_version_satisfied() {
            // panicking here is silly...
            panic!("{}", msg);
        }
        util::CargoCommand::new(subcommand)
            .with_package(Some(config.app_name()))
            .with_manifest_path(Some(config.manifest_path()))
            .with_target(Some(&self.triple))
            .with_features(Some("metal"))
    }

    pub fn check(&self, config: &Config, env: &Env, noise_level: NoiseLevel) {
        self.cargo(config, "check")
            .with_verbose(noise_level.is_pedantic())
            .into_command(env)
            .status()
            .into_result()
            .expect("Failed to run `cargo check`");
    }

    pub fn compile_lib(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) {
        // NOTE: it's up to Xcode to pass the verbose flag here, so even when
        // using our build/run commands it won't get passed.
        // TODO: I don't undestand this comment
        self.cargo(config, "build")
            .with_verbose(noise_level.is_pedantic())
            .with_release(profile.is_release())
            .into_command(env)
            .status()
            .into_result()
            .expect("Failed to run `cargo build`");
    }

    pub fn build(&self, config: &Config, env: &Env, profile: Profile) {
        let configuration = profile.as_str();
        PureCommand::new("xcodebuild", env)
            .args(&["-scheme", &config.ios().scheme()])
            .arg("-workspace")
            .arg(&config.ios().workspace_path())
            .args(&["-configuration", configuration])
            .args(&["-arch", self.arch])
            .arg("build")
            .status()
            .into_result()
            .expect("Failed to run `xcodebuild`");
    }

    fn archive(&self, config: &Config, env: &Env, profile: Profile) {
        let configuration = profile.as_str();
        let archive_path = config.ios().export_path().join(&config.ios().scheme());
        PureCommand::new("xcodebuild", env)
            .args(&["-scheme", &config.ios().scheme()])
            .arg("-workspace")
            .arg(&config.ios().workspace_path())
            .args(&["-sdk", "iphoneos"])
            .args(&["-configuration", configuration])
            .args(&["-arch", self.arch])
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
        PureCommand::new("xcodebuild", env)
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

    pub fn run(&self, config: &Config, env: &Env, profile: Profile) {
        // TODO: These steps are run unconditionally, which is slooooooow
        self.build(config, env, profile);
        self.archive(config, env, profile);
        PureCommand::new("unzip", env)
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
        ios_deploy(env)
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
