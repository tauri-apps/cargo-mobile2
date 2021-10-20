use super::{
    adb, bundletool,
    config::Config,
    env::Env,
    jnilibs::{self, JniLibs},
    target::{BuildError, Target},
};
use crate::{
    env::ExplicitEnv as _,
    opts::{self, FilterLevel, NoiseLevel, Profile},
    util::{
        self,
        cli::{Report, Reportable},
    },
};
use std::{
    fmt::{self, Display},
    path::PathBuf,
};

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
pub enum AabBuildError {
    BuildFailed(bossy::Error),
}

impl Reportable for AabBuildError {
    fn report(&self) -> Report {
        match self {
            Self::BuildFailed(err) => Report::error("Failed to build AAB", err),
        }
    }
}

#[derive(Debug)]
pub enum ApksBuildError {
    CleanFailed(std::io::Error),
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

#[derive(Debug)]
pub enum ApkInstallError {
    InstallFailed(bossy::Error),
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

#[derive(Debug)]
pub enum RunError {
    ApkBuildFailed(ApkBuildError),
    ApkInstallFailed(ApkInstallError),
    StartFailed(bossy::Error),
    WakeScreenFailed(bossy::Error),
    LogcatFailed(bossy::Error),
    BundletoolInstallFailed(bundletool::InstallError),
    AabBuildFailed(AabBuildError),
    ApksFromAabBuildFailed(ApksBuildError),
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::ApkBuildFailed(err) => err.report(),
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

    fn adb(&self, env: &Env) -> bossy::Command {
        adb::adb(env, &self.serial_no)
    }

    fn suffix(profile: Profile) -> &'static str {
        match profile {
            Profile::Debug => profile.as_str(),
            // TODO: how to handle signed APKs?
            Profile::Release => "release-unsigned",
        }
    }

    fn output_resource_path(
        output_dir: String,
        file_extension: &str,
        config: &Config,
        profile: Profile,
        flavor: &str,
    ) -> PathBuf {
        let suffix = Self::suffix(profile);
        config.project_dir().join(format!(
            "app/build/outputs/{}/app-{}-{}.{}",
            output_dir, flavor, suffix, file_extension
        ))
    }

    fn apk_path(config: &Config, profile: Profile, flavor: &str) -> PathBuf {
        Self::output_resource_path(
            format!("apk/{}/{}", flavor, profile.as_str()),
            "apk",
            config,
            profile,
            flavor,
        )
    }

    fn apks_path(config: &Config, profile: Profile, flavor: &str) -> PathBuf {
        Self::output_resource_path(
            format!("apk/{}/{}", flavor, profile.as_str()),
            "apks",
            config,
            profile,
            flavor,
        )
    }

    fn aab_path(config: &Config, profile: Profile, flavor: &str) -> PathBuf {
        Self::output_resource_path(
            format!("bundle/{}{}", flavor, profile.as_str()),
            "aab",
            config,
            profile,
            flavor,
        )
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
        let apk_path = Self::apk_path(config, profile, flavor);
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

    fn build_aab(&self, config: &Config, env: &Env, profile: Profile) -> Result<(), AabBuildError> {
        use heck::CamelCase as _;
        let flavor = self.target.arch.to_camel_case();
        let build_ty = profile.as_str().to_camel_case();
        gradlew(config, env)
            .with_arg(format!(":app:bundle{}{}", flavor, build_ty))
            .run_and_wait()
            .map_err(AabBuildError::BuildFailed)?;
        Ok(())
    }

    fn build_apks_from_aab(&self, config: &Config, profile: Profile) -> Result<(), ApksBuildError> {
        let flavor = self.target.arch;
        let apks_path = Self::apks_path(config, profile, flavor);
        let aab_path = Self::aab_path(config, profile, flavor);
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
        reinstall_deps: opts::ReinstallDeps,
    ) -> Result<(), RunError> {
        if build_app_bundle {
            bundletool::install(reinstall_deps).map_err(RunError::BundletoolInstallFailed)?;
            self.clean_apks(config, profile)
                .map_err(RunError::ApksFromAabBuildFailed)?;
            self.build_aab(config, env, profile)
                .map_err(RunError::AabBuildFailed)?;
            self.build_apks_from_aab(config, profile)
                .map_err(RunError::ApksFromAabBuildFailed)?;
            self.install_apk_from_aab(config, profile)
                .map_err(RunError::ApkInstallFailed)?;
        } else {
            self.build_apk(config, env, noise_level, profile)
                .map_err(RunError::ApkBuildFailed)?;
            self.install_apk(config, env, profile)
                .map_err(RunError::ApkInstallFailed)?;
        }
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
