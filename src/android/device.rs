use super::{
    adb,
    config::Config,
    env::Env,
    target::{BuildError, Target},
};
use crate::{env::ExplicitEnv as _, opts::Profile, util};
use std::{
    fmt::{self, Display},
    io,
};
use structexec::NoiseLevel;

fn gradlew(config: &Config, env: &Env) -> bossy::Command {
    let gradlew_path = config.project_dir().join("gradlew");
    bossy::Command::pure(&gradlew_path)
        .with_env_vars(env.explicit_env())
        .with_arg("--project-dir")
        .with_arg(config.project_dir())
}

#[derive(Debug)]
pub enum ApkBuildError {
    LibSymlinkCleaningFailed(io::Error),
    LibBuildFailed(BuildError),
    AssembleFailed(bossy::Error),
}

impl Display for ApkBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LibSymlinkCleaningFailed(err) => {
                write!(f, "Failed to delete broken symlink: {}", err)
            }
            Self::LibBuildFailed(err) => write!(f, "{}", err),
            Self::AssembleFailed(err) => write!(f, "Failed to assemble APK: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum ApkInstallError {
    InstallFailed(bossy::Error),
}

impl Display for ApkInstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InstallFailed(err) => write!(f, "Failed to install APK: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum RunError {
    ApkBuildFailed(ApkBuildError),
    ApkInstallFailed(ApkInstallError),
    StartFailed(bossy::Error),
    WakeScreenFailed(bossy::Error),
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

impl Display for StacktraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PipeFailed(err) => write!(f, "Failed to pipe stacktrace output: {}", err),
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

impl<'a> Display for Device<'a> {
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

    fn adb(&self, env: &Env) -> bossy::Command {
        adb::adb(env, &self.serial_no)
    }

    fn build_apk(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<(), ApkBuildError> {
        use heck::CamelCase as _;
        Target::clean_jnilibs(config).map_err(ApkBuildError::LibSymlinkCleaningFailed)?;
        let flavor = self.target.arch.to_camel_case();
        let build_ty = profile.as_str().to_camel_case();
        gradlew(config, env)
            .with_arg(format!("assemble{}{}", flavor, build_ty))
            .with_arg(match noise_level {
                NoiseLevel::Polite => "--warn",
                NoiseLevel::LoudAndProud => "--info",
                NoiseLevel::FranklyQuitePedantic => "--debug",
            })
            .run_and_wait()
            .map_err(ApkBuildError::AssembleFailed)?;
        Ok(())
    }

    fn install_apk(
        &self,
        config: &Config,
        env: &Env,
        profile: Profile,
    ) -> Result<(), ApkInstallError> {
        let flavor = self.target.arch;
        let build_ty = profile.as_str();
        let apk_path = config.project_dir().join(format!(
            "app/build/outputs/apk/{}/{}/app-{}-{}.apk",
            flavor, build_ty, flavor, build_ty
        ));
        self.adb(env)
            .with_arg("install")
            .with_arg(apk_path)
            .run_and_wait()
            .map_err(ApkInstallError::InstallFailed)?;
        Ok(())
    }

    fn wake_screen(&self, env: &Env) -> bossy::Result<()> {
        self.adb(env)
            .with_args(&["shell", "input", "keyevent", "KEYCODE_WAKEUP"])
            .run_and_wait()?;
        Ok(())
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
        self.install_apk(config, env, profile)
            .map_err(RunError::ApkInstallFailed)?;
        let activity = format!(
            "{}.{}/android.app.NativeActivity",
            config.app().reverse_domain(),
            config.app().name_snake(),
        );
        self.adb(env)
            .with_args(&["shell", "am", "start", "-n", &activity])
            .run_and_wait()
            .map_err(RunError::StartFailed)?;
        self.wake_screen(env).map_err(RunError::WakeScreenFailed)
    }

    pub fn stacktrace(&self, config: &Config, env: &Env) -> Result<(), StacktraceError> {
        // -d = print and exit
        let logcat_command = adb::adb(env, &self.serial_no).with_args(&["logcat", "-d"]);
        let stack_command = bossy::Command::pure("ndk-stack")
            .with_env_vars(env.explicit_env())
            .with_env_var("PATH", util::add_to_path(env.ndk.home().display()))
            .with_arg("-sym")
            .with_arg(
                config
                    .app()
                    // ndk-stack can't seem to handle spaces in args, no matter
                    // how I try to quote or escape them... so, instead of
                    // mandating that the entire path not contain spaces, we'll
                    // just use a relative path!
                    .unprefix_path(self.target.get_jnilibs_subdir(config))
                    .expect("developer error: jnilibs subdir not prefixed"),
            );
        if !util::pipe(logcat_command, stack_command).map_err(StacktraceError::PipeFailed)? {
            println!("  -- no stacktrace --");
        }
        Ok(())
    }
}
