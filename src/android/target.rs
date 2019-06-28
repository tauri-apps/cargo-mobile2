// TODO: Bad things happen if multiple Android devices are connected at once

use crate::{
    init::CargoTarget,
    target::{get_possible_values, TargetTrait},
    util::{self, force_symlink, IntoResult},
    CONFIG,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
    process::Command,
};

const API_VERSION: u32 = 24;

lazy_static::lazy_static! {
    pub static ref POSSIBLE_TARGETS: Vec<&'static str> = { get_possible_values::<Target>() };
    static ref NDK_HOME: String = env::var("NDK_HOME").expect("`NDK_HOME` env var missing");
}

fn gradlew() -> Command {
    let gradlew_path = CONFIG.android.project_path().join("gradlew");
    let mut command = Command::new(&gradlew_path);
    command.arg("--project-dir");
    command.arg(CONFIG.android.project_path());
    command
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Target {
    pub triple: String,
    pub abi: String,
    pub arch: String,
}

impl TargetTrait for Target {
    fn all() -> &'static BTreeMap<String, Self> {
        &CONFIG.android.targets
    }
    fn triple(&self) -> &str {
        &self.triple
    }
    fn arch(&self) -> &str {
        &self.arch
    }
}

impl Target {
    fn for_abi(abi: &str) -> Option<&'static Self> {
        Self::all().values().find(|target| target.abi == abi)
    }

    pub fn for_connected() -> util::CommandResult<Option<&'static Self>> {
        let output = Command::new("adb")
            .args(&["shell", "getprop", "ro.product.cpu.abi"])
            .output()
            .into_result()?;
        let raw_abi = String::from_utf8(output.stdout)
            .expect("`ro.product.cpu.abi` contained invalid unicode");
        let abi = raw_abi.trim();
        Ok(Self::for_abi(abi))
    }

    fn bin_path(&self, bin: &str) -> String {
        CONFIG
            .android
            .ndk_path()
            .join(format!("{}/bin/{}-{}", self.arch, self.triple, bin))
            .to_str()
            .expect("NDK path contained invalid unicode")
            .to_owned()
    }

    pub fn get_cargo_config(&self) -> CargoTarget {
        let ar = CONFIG
            .unprefix_path(self.bin_path("ar"))
            .to_str()
            .expect("Archiver path contained invalid unicode")
            .to_owned();
        let linker = CONFIG
            .unprefix_path(self.bin_path("clang"))
            .to_str()
            .expect("Linker path contained invalid unicode")
            .to_owned();
        CargoTarget {
            ar: Some(ar),
            linker: Some(linker),
            rustflags: vec![
                "-C".to_owned(),
                "link-arg=-landroid".to_owned(),
                "-C".to_owned(),
                "link-arg=-llog".to_owned(),
                "-C".to_owned(),
                "link-arg=-lOpenSLES".to_owned(),
            ],
        }
    }

    // Add clang/gcc binaries to PATH
    fn add_arch_to_path(&self) -> String {
        let path = CONFIG
            .android
            .ndk_path()
            .join(format!("{}/bin", &self.arch))
            .canonicalize()
            .expect("Failed to canonicalize toolchain path");
        util::add_to_path(path.to_str().unwrap())
    }

    fn compile_lib(&self, verbose: bool, release: bool, check: bool) {
        let subcommand = if check { "check" } else { "build" };
        util::CargoCommand::new(subcommand)
            .with_verbose(verbose)
            .with_package(Some(CONFIG.app_name()))
            .with_manifest_path(CONFIG.manifest_path())
            .with_target(Some(&self.triple))
            .with_features(Some("vulkan"))
            .with_release(release)
            .into_command()
            .env("PATH", self.add_arch_to_path())
            .status()
            .into_result()
            .expect("Failed to run `cargo build`");
    }

    fn get_jnilibs_subdir(&self) -> PathBuf {
        CONFIG
            .android
            .project_path()
            .join(format!("app/src/main/jniLibs/{}", &self.abi))
    }

    fn make_jnilibs_subdir(&self) -> Result<(), io::Error> {
        let path = self.get_jnilibs_subdir();
        fs::create_dir_all(path)
    }

    fn symlink_lib(&self) {
        self.make_jnilibs_subdir()
            .expect("Failed to create jniLibs subdir");
        let so_name = format!("lib{}.so", CONFIG.app_name());
        let src = CONFIG.prefix_path(format!("target/{}/debug/{}", &self.triple, &so_name));
        if !src.exists() {
            panic!("Symlink source doesn't exist: {:?}", src);
        }
        let dest = self.get_jnilibs_subdir().join(&so_name);
        force_symlink(src, dest).expect("Failed to symlink lib");
    }

    pub fn check(&self, verbose: bool) {
        self.build_toolchain();
        self.compile_lib(verbose, false, true);
    }

    fn build_toolchain(&self) {
        let toolchains_dir = CONFIG.android.ndk_path();
        let arch_dir = toolchains_dir.join(&self.arch);
        if !arch_dir.exists() {
            fs::create_dir_all(&toolchains_dir).expect("Failed to create toolchain directory");
            let ndk_home = Path::new(&*NDK_HOME);
            Command::new(ndk_home.join("build/tools/make_standalone_toolchain.py"))
                .args(&["--api", &API_VERSION.to_string()])
                .args(&["--arch", &self.arch])
                .args(&["--install-dir", arch_dir.to_str().unwrap()])
                .status()
                .into_result()
                .expect("Failed to build toolchain");
        }
    }

    pub fn build(&self, verbose: bool, release: bool) {
        self.build_toolchain();
        self.compile_lib(verbose, release, false);
        self.symlink_lib();
    }

    fn build_and_install(&self, verbose: bool, release: bool) {
        self.build(verbose, release);
        gradlew()
            .arg("installDebug")
            .status()
            .into_result()
            .expect("Failed to build and install APK");
    }

    fn wake_screen(&self) {
        Command::new("adb")
            .args(&["shell", "input", "keyevent", "KEYCODE_WAKEUP"])
            .status()
            .into_result()
            .expect("Failed to wake device screen");
    }

    pub fn run(&self, verbose: bool, release: bool) {
        self.build_and_install(verbose, release);
        let activity = format!(
            "{}.{}/android.app.NativeActivity",
            CONFIG.reverse_domain(),
            CONFIG.app_name(),
        );
        Command::new("adb")
            .args(&["shell", "am", "start", "-n", &activity])
            .status()
            .into_result()
            .expect("Failed to start APK on device");
        self.wake_screen();
    }

    pub fn stacktrace(&self) {
        let mut logcat_command = Command::new("adb");
        logcat_command.args(&["logcat", "-d"]); // print and exit
        let mut stack_command = Command::new("ndk-stack");
        stack_command
            .env("PATH", util::add_to_path(&*NDK_HOME))
            .arg("-sym")
            .arg(self.get_jnilibs_subdir());
        util::pipe(logcat_command, stack_command).expect("Failed to get stacktrace");
    }
}
