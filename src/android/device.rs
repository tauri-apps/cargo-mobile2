use super::{
    adb,
    config::Config,
    env::Env,
    jnilibs::{self, JniLibs},
    target::{BuildError, Target},
};
use crate::{
    env::ExplicitEnv as _,
    opts::{FilterLevel, NoiseLevel, Profile},
    util::{
        self,
        cli::{Report, Reportable},
    },
};
use std::fmt::{self, Display};

fn gradlew(config: &Config, env: &Env) -> bossy::Command {
    let gradlew_path = config.project_dir().join("gradlew");
    bossy::Command::pure(&gradlew_path)
        .with_env_vars(env.explicit_env())
        .with_arg("--project-dir")
        .with_arg(config.project_dir())
}

#[derive(Debug)]
pub enum ApkBuildError {
    LibSymlinkCleaningFailed(jnilibs::RemoveBrokenLinksError),
    LibBuildFailed(BuildError),
    AssembleFailed(bossy::Error),
}

impl Reportable for ApkBuildError {
    fn report(&self) -> Report {
        match self {
            Self::LibSymlinkCleaningFailed(err) => err.report(),
            Self::LibBuildFailed(err) => err.report(),
            Self::AssembleFailed(err) => Report::error("Failed to assemble APK", err),
        }
    }
}

#[derive(Debug)]
pub enum ApkInstallError {
    InstallFailed(bossy::Error),
}

impl Reportable for ApkInstallError {
    fn report(&self) -> Report {
        match self {
            Self::InstallFailed(err) => Report::error("Failed to install APK", err),
        }
    }
}

#[derive(Debug)]
pub enum RunError {
    ApkBuildFailed(ApkBuildError),
    ApkInstallFailed(ApkInstallError),
    StartFailed(bossy::Error),
    WakeScreenFailed(bossy::Error),
    LogcatFailed(bossy::Error),
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::ApkBuildFailed(err) => err.report(),
            Self::ApkInstallFailed(err) => err.report(),
            Self::StartFailed(err) => Report::error("Failed to start app on device", err),
            Self::WakeScreenFailed(err) => Report::error("Failed to wake device screen", err),
            Self::LogcatFailed(err) => Report::error("Failed to log output", err),
        }
    }
}

#[derive(Debug)]
pub enum StacktraceError {
    PipeFailed(util::PipeError),
}

impl Reportable for StacktraceError {
    fn report(&self) -> Report {
        match self {
            Self::PipeFailed(err) => Report::error("Failed to pipe stacktrace output", err),
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
        JniLibs::remove_broken_links(config).map_err(ApkBuildError::LibSymlinkCleaningFailed)?;
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
        let apk_suffix = match profile {
            Profile::Debug => build_ty,
            // TODO: how to handle signed APKs?
            Profile::Release => "release-unsigned",
        };
        let apk_path = config.project_dir().join(format!(
            "app/build/outputs/apk/{}/{}/app-{}-{}.apk",
            flavor, build_ty, flavor, apk_suffix
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
        filter_level: Option<FilterLevel>,
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
        self.wake_screen(env).map_err(RunError::WakeScreenFailed)?;
        let filter = format!(
            "{}:{}",
            config.app().name(),
            filter_level
                .unwrap_or(match noise_level {
                    NoiseLevel::Polite => FilterLevel::Warn,
                    NoiseLevel::LoudAndProud => FilterLevel::Info,
                    NoiseLevel::FranklyQuitePedantic => FilterLevel::Verbose,
                })
                .logcat()
        );
        adb::adb(env, &self.serial_no)
            .with_args(&["logcat", "-v", "color", "-s", &filter])
            .run_and_wait()
            .map_err(RunError::LogcatFailed)?;
        Ok(())
    }

    pub fn stacktrace(&self, config: &Config, env: &Env) -> Result<(), StacktraceError> {
        // -d = print and exit
        let logcat_command = adb::adb(env, &self.serial_no).with_args(&["logcat", "-d"]);
        let stack_command = bossy::Command::pure("ndk-stack")
            .with_env_vars(env.explicit_env())
            .with_env_var(
                "PATH",
                util::prepend_to_path(env.ndk.home().display(), env.path()),
            )
            .with_arg("-sym")
            .with_arg(
                config
                    .app()
                    // ndk-stack can't seem to handle spaces in args, no matter
                    // how I try to quote or escape them... so, instead of
                    // mandating that the entire path not contain spaces, we'll
                    // just use a relative path!
                    .unprefix_path(jnilibs::path(config, *self.target))
                    .expect("developer error: jnilibs subdir not prefixed"),
            );
        if !util::pipe(logcat_command, stack_command).map_err(StacktraceError::PipeFailed)? {
            println!("  -- no stacktrace --");
        }
        Ok(())
    }
}
