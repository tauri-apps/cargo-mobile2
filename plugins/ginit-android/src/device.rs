use crate::{
    adb,
    config::Config,
    env::Env,
    target::{BuildError, Target},
};
use ginit_core::{
    config::ConfigTrait,
    exports::into_result::{command::CommandError, IntoResult as _},
    opts::{NoiseLevel, Profile},
    util::{self, pure_command::PureCommand},
};
use std::{fmt, io, process::Command};

fn gradlew(config: &Config, env: &Env) -> Command {
    let gradlew_path = config.project_path().join("gradlew");
    let mut command = PureCommand::new(&gradlew_path, env);
    command.arg("--project-dir");
    command.arg(config.project_path());
    command
}

#[derive(Debug)]
pub enum ApkBuildError {
    LibSymlinkCleaningFailed(io::Error),
    LibBuildFailed(BuildError),
    AssembleFailed(CommandError),
}

impl fmt::Display for ApkBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApkBuildError::LibSymlinkCleaningFailed(err) => {
                write!(f, "Failed to delete broken symlink: {}", err)
            }
            ApkBuildError::LibBuildFailed(err) => write!(f, "{}", err),
            ApkBuildError::AssembleFailed(err) => write!(f, "Failed to assemble APK: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum ApkInstallError {
    InstallFailed(CommandError),
}

impl fmt::Display for ApkInstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApkInstallError::InstallFailed(err) => write!(f, "Failed to install APK: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum RunError {
    ApkBuildFailed(ApkBuildError),
    ApkInstallFailed(ApkInstallError),
    StartFailed(CommandError),
    WakeScreenFailed(CommandError),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::ApkBuildFailed(err) => write!(f, "Failed to build app: {}", err),
            RunError::ApkInstallFailed(err) => write!(f, "Failed to install app: {}", err),
            RunError::StartFailed(err) => write!(f, "Failed to start app on device: {}", err),
            RunError::WakeScreenFailed(err) => write!(f, "Failed to wake device screen: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum StacktraceError {
    PipeFailed(util::PipeError),
}

impl fmt::Display for StacktraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StacktraceError::PipeFailed(err) => {
                write!(f, "Failed to pipe stacktrace output: {}", err)
            }
        }
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Device<'a> {
    serial_no: String,
    name: String,
    model: String,
    target: &'a Target<'a>,
}

impl<'a> fmt::Display for Device<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if self.model != self.name {
            write!(f, " ({})", self.model)?;
        }
        Ok(())
    }
}

impl<'a> Device<'a> {
    pub(super) fn new(
        serial_no: String,
        name: String,
        model: String,
        target: &'a Target<'a>,
    ) -> Self {
        Self {
            serial_no,
            name,
            model,
            target,
        }
    }

    pub fn target(&self) -> &'a Target<'a> {
        self.target
    }

    fn adb(&self, env: &Env) -> Command {
        adb::adb(env, &self.serial_no)
    }

    fn build_apk(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<(), ApkBuildError> {
        Target::clean_jnilibs(config).map_err(ApkBuildError::LibSymlinkCleaningFailed)?;
        self.target
            .build(config, env, noise_level, profile)
            .map_err(ApkBuildError::LibBuildFailed)?;
        gradlew(config, env)
            .arg("assembleDebug")
            .status()
            .into_result()
            .map_err(ApkBuildError::AssembleFailed)
    }

    fn install_apk(&self, config: &Config, env: &Env) -> Result<(), ApkInstallError> {
        let apk_path = config
            .project_path()
            .join("app/build/outputs/apk/debug/app-debug.apk");
        self.adb(env)
            .arg("install")
            .arg(apk_path)
            .status()
            .into_result()
            .map_err(ApkInstallError::InstallFailed)
    }

    fn wake_screen(&self, env: &Env) -> Result<(), CommandError> {
        self.adb(env)
            .args(&["shell", "input", "keyevent", "KEYCODE_WAKEUP"])
            .status()
            .into_result()
    }

    pub fn run(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<(), RunError> {
        self.build_apk(config, env, noise_level, profile)
            .map_err(RunError::ApkBuildFailed)?;
        self.install_apk(config, env)
            .map_err(RunError::ApkInstallFailed)?;
        let activity = format!(
            "{}.{}/android.app.NativeActivity",
            config.shared().reverse_domain(),
            config.shared().app_name_snake(),
        );
        self.adb(env)
            .args(&["shell", "am", "start", "-n", &activity])
            .status()
            .into_result()
            .map_err(RunError::StartFailed)?;
        self.wake_screen(env).map_err(RunError::WakeScreenFailed)
    }

    pub fn stacktrace(&self, config: &Config, env: &Env) -> Result<(), StacktraceError> {
        let mut logcat_command = adb::adb(env, &self.serial_no);
        logcat_command.args(&["logcat", "-d"]); // print and exit
        let mut stack_command = PureCommand::new("ndk-stack", env);
        stack_command
            .env("PATH", util::add_to_path(env.ndk.home().display()))
            .arg("-sym")
            .arg(self.target.get_jnilibs_subdir(config));
        util::pipe(logcat_command, stack_command).map_err(StacktraceError::PipeFailed)
    }
}
