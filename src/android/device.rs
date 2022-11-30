use super::{aab, adb, bundletool, config::Config, env::Env, jnilibs, target::Target};
use crate::{
    android::apk,
    bossy,
    env::ExplicitEnv as _,
    opts::{FilterLevel, NoiseLevel, Profile},
    os::consts,
    util::{
        self,
        cli::{Report, Reportable},
        prefix_path,
    },
};
use bossy::Handle;
use std::{
    fmt::{self, Display},
    path::PathBuf,
    thread::sleep,
    time::Duration,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AabBuildError {
    #[error("Failed to build AAB: {0}")]
    BuildFailed(bossy::Error),
}

impl Reportable for AabBuildError {
    fn report(&self) -> Report {
        match self {
            Self::BuildFailed(err) => Report::error("Failed to build AAB", err),
        }
    }
}

#[derive(Debug, Error)]
pub enum ApksBuildError {
    #[error("Failed to clean old APKS: {0}")]
    CleanFailed(std::io::Error),
    #[error("Failed to build APKS from AAB: {0}")]
    BuildFromAabFailed(bossy::Error),
}

impl Reportable for ApksBuildError {
    fn report(&self) -> Report {
        match self {
            Self::CleanFailed(err) => Report::error("Failed to clean old APKS", err),
            Self::BuildFromAabFailed(err) => Report::error("Failed to build APKS from AAB", err),
        }
    }
}

#[derive(Debug, Error)]
pub enum ApkInstallError {
    #[error("Failed to install APK: {0}")]
    InstallFailed(bossy::Error),
    #[error("Failed to install APK from AAB: {0}")]
    InstallFromAabFailed(bossy::Error),
}

impl Reportable for ApkInstallError {
    fn report(&self) -> Report {
        match self {
            Self::InstallFailed(err) => Report::error("Failed to install APK", err),
            Self::InstallFromAabFailed(err) => Report::error("Failed to install APK from AAB", err),
        }
    }
}

#[derive(Debug, Error)]
pub enum RunError {
    #[error(transparent)]
    ApkError(apk::ApkError),
    #[error(transparent)]
    AabError(aab::AabError),
    #[error(transparent)]
    ApkInstallFailed(ApkInstallError),
    #[error("Failed to start app on device: {0}")]
    StartFailed(bossy::Error),
    #[error("Failed to wake device screen: {0}")]
    WakeScreenFailed(bossy::Error),
    #[error("Failed to log output: {0}")]
    LogcatFailed(bossy::Error),
    #[error(transparent)]
    BundletoolInstallFailed(bundletool::InstallError),
    #[error(transparent)]
    AabBuildFailed(AabBuildError),
    #[error(transparent)]
    ApksFromAabBuildFailed(ApksBuildError),
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::ApkError(err) => err.report(),
            Self::AabError(err) => err.report(),
            Self::ApkInstallFailed(err) => err.report(),
            Self::StartFailed(err) => Report::error("Failed to start app on device", err),
            Self::WakeScreenFailed(err) => Report::error("Failed to wake device screen", err),
            Self::LogcatFailed(err) => Report::error("Failed to log output", err),
            Self::BundletoolInstallFailed(err) => err.report(),
            Self::AabBuildFailed(err) => err.report(),
            Self::ApksFromAabBuildFailed(err) => err.report(),
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    fn adb(&self, env: &Env) -> bossy::Command {
        adb::adb(env, &self.serial_no)
    }

    fn apks_path(config: &Config, profile: Profile, flavor: &str) -> PathBuf {
        prefix_path(
            config.project_dir(),
            format!(
                "app/build/outputs/{}/app-{}-{}.{}",
                format!("apk/{}/{}", flavor, profile.as_str()),
                flavor,
                profile.suffix(),
                "apks"
            ),
        )
    }

    fn wait_device_boot(&self, env: &Env) {
        loop {
            if let Ok(output) = self
                .adb(env)
                .with_args(&["shell", "getprop", "init.svc.bootanim"])
                .run_and_wait_for_string()
            {
                if output.trim() == "stopped" {
                    break;
                }
                sleep(Duration::from_secs(2));
            } else {
                break;
            }
        }
    }

    fn build_apk(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<(), apk::ApkError> {
        apk::build(config, env, noise_level, profile, vec![self.target()], true)?;
        Ok(())
    }

    fn install_apk(
        &self,
        config: &Config,
        env: &Env,
        profile: Profile,
    ) -> Result<(), ApkInstallError> {
        let flavor = self.target.arch;
        let apk_path = apk::apk_path(config, profile, flavor);
        self.adb(env)
            .with_arg("install")
            .with_arg(apk_path)
            .run_and_wait()
            .map_err(ApkInstallError::InstallFailed)?;
        Ok(())
    }

    fn clean_apks(&self, config: &Config, profile: Profile) -> Result<(), ApksBuildError> {
        let flavor = self.target.arch;
        let apks_path = Self::apks_path(config, profile, flavor);
        if apks_path.exists() {
            std::fs::remove_file(&apks_path).map_err(ApksBuildError::CleanFailed)?;
        }
        Ok(())
    }

    fn build_aab(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<(), aab::AabError> {
        aab::build(
            config,
            env,
            noise_level,
            profile,
            vec![self.target()],
            false,
        )?;
        Ok(())
    }

    fn build_apks_from_aab(&self, config: &Config, profile: Profile) -> Result<(), ApksBuildError> {
        let flavor = self.target.arch;
        let apks_path = Self::apks_path(config, profile, flavor);
        let aab_path = aab::aab_path(config, profile, flavor);
        bundletool::command()
            .with_arg("build-apks")
            .with_arg(format!("--bundle={}", aab_path.to_str().unwrap()))
            .with_arg(format!("--output={}", apks_path.to_str().unwrap()))
            .with_arg("--connected-device")
            .run_and_wait()
            .map_err(ApksBuildError::BuildFromAabFailed)?;
        Ok(())
    }

    fn install_apk_from_aab(
        &self,
        config: &Config,
        profile: Profile,
    ) -> Result<(), ApkInstallError> {
        let flavor = self.target.arch;
        let apks_path = Self::apks_path(config, profile, flavor);
        bundletool::command()
            .with_arg("install-apks")
            .with_arg(format!("--apks={}", apks_path.to_str().unwrap()))
            .run_and_wait()
            .map_err(ApkInstallError::InstallFromAabFailed)?;
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
        build_app_bundle: bool,
        reinstall_deps: bool,
        activity: String,
    ) -> Result<Handle, RunError> {
        if build_app_bundle {
            bundletool::install(reinstall_deps).map_err(RunError::BundletoolInstallFailed)?;
            self.clean_apks(config, profile)
                .map_err(RunError::ApksFromAabBuildFailed)?;
            self.build_aab(config, env, noise_level, profile)
                .map_err(RunError::AabError)?;
            self.build_apks_from_aab(config, profile)
                .map_err(RunError::ApksFromAabBuildFailed)?;
            if self.serial_no.starts_with("emulator") {
                self.wait_device_boot(env);
            }
            self.install_apk_from_aab(config, profile)
                .map_err(RunError::ApkInstallFailed)?;
        } else {
            self.build_apk(config, env, noise_level, profile)
                .map_err(RunError::ApkError)?;
            if self.serial_no.starts_with("emulator") {
                self.wait_device_boot(env);
            }
            self.install_apk(config, env, profile)
                .map_err(RunError::ApkInstallFailed)?;
        }
        let activity = format!(
            "{}.{}/{}",
            config.app().reverse_domain(),
            config.app().name_snake(),
            activity
        );
        self.adb(env)
            .with_args(&["shell", "am", "start", "-n", &activity])
            .run_and_wait()
            .map_err(RunError::StartFailed)?;
        let _ = self.wake_screen(env);

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

        let out = bossy::Command::pure(env.platform_tools_path().join("adb"))
            .with_env_vars(env.explicit_env())
            .with_args(&[
                "shell",
                "pidof",
                "-s",
                &format!(
                    "{}.{}",
                    config.app().reverse_domain(),
                    config.app().name_snake(),
                ),
            ])
            .run_and_wait_for_output()
            .map_err(RunError::LogcatFailed)?;
        let pid = out.stdout_str().map(|p| p.trim()).unwrap_or_default();
        let mut logcat =
            bossy::Command::pure(env.platform_tools_path().join("adb")).with_arg("logcat");
        if !pid.is_empty() {
            logcat = logcat.with_args(&["--pid", pid]);
        }
        logcat
            .with_env_vars(env.explicit_env())
            .with_args(&["-v", "color", "-s", &filter])
            .with_args(config.logcat_filter_specs())
            .run()
            .map_err(RunError::LogcatFailed)
    }

    pub fn stacktrace(&self, config: &Config, env: &Env) -> Result<(), StacktraceError> {
        // -d = print and exit
        let logcat_command = adb::adb(env, &self.serial_no).with_args(&["logcat", "-d"]);
        let stack_command = bossy::Command::pure(env.ndk.home().join(consts::NDK_STACK))
            .with_env_vars(env.explicit_env())
            .with_env_var(
                "PATH",
                util::prepend_to_path(env.ndk.home().display(), env.path().to_string_lossy()),
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
