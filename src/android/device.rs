use super::{aab, adb, bundletool, config::Config, env::Env, jnilibs, target::Target};
use crate::{
    android::apk,
    env::ExplicitEnv as _,
    opts::{FilterLevel, NoiseLevel, Profile},
    os::consts,
    util::{
        self,
        cli::{Report, Reportable},
        last_modified, prefix_path,
    },
    DuctExpressionExt,
};
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
    BuildFailed(std::io::Error),
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
    BuildFromAabFailed(std::io::Error),
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
    InstallFailed(#[from] std::io::Error),
    #[error("Failed to install APK from AAB: {0}")]
    InstallFromAabFailed(std::io::Error),
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
    #[error("Failed to wake device screen: {0}")]
    WakeScreenFailed(std::io::Error),
    #[error(transparent)]
    BundletoolInstallFailed(bundletool::InstallError),
    #[error(transparent)]
    AabBuildFailed(AabBuildError),
    #[error(transparent)]
    ApksFromAabBuildFailed(ApksBuildError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::ApkError(err) => err.report(),
            Self::AabError(err) => err.report(),
            Self::ApkInstallFailed(err) => err.report(),
            Self::WakeScreenFailed(err) => Report::error("Failed to wake device screen", err),
            Self::BundletoolInstallFailed(err) => err.report(),
            Self::AabBuildFailed(err) => err.report(),
            Self::ApksFromAabBuildFailed(err) => err.report(),
            Self::Io(err) => Report::error("IO error", err),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StacktraceError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Reportable for StacktraceError {
    fn report(&self) -> Report {
        match self {
            Self::Io(err) => Report::error("IO error", err),
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

    fn adb(&self, env: &Env) -> duct::Expression {
        adb::adb(env, &self.serial_no)
    }

    pub fn all_apks_paths(config: &Config, profile: Profile, flavor: &str) -> Vec<PathBuf> {
        profile
            .suffixes()
            .iter()
            .map(|suffix| {
                prefix_path(
                    config.project_dir(),
                    format!(
                        "app/build/outputs/apk/{}/{}/app-{}-{}.{}",
                        flavor,
                        profile.as_str(),
                        flavor,
                        suffix,
                        "apk"
                    ),
                )
            })
            .collect()
    }

    fn wait_device_boot(&self, env: &Env) {
        loop {
            let cmd = self
                .adb(env)
                .stderr_capture()
                .stdout_capture()
                .before_spawn(move |cmd| {
                    cmd.args(["shell", "getprop", "init.svc.bootanim"]);
                    Ok(())
                });
            let handle = cmd.start();
            if let Ok(handle) = handle {
                if let Ok(output) = handle.wait() {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if stdout.trim() == "stopped" {
                            break;
                        }
                        sleep(Duration::from_secs(2));
                    }
                } else {
                    break;
                }
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
        let apk_path = apk::apks_paths(config, profile, flavor)
            .into_iter()
            .reduce(last_modified)
            .unwrap();

        self.adb(env)
            .before_spawn(move |cmd| {
                cmd.args(["install", "-r"]);
                cmd.arg(&apk_path);
                Ok(())
            })
            .dup_stdio()
            .start()?
            .wait()?;

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
        // In the case that profile is `Release`, it is safe to pick the first one
        // which should have the suffix `release` instead of `release-unsigned`.
        // This is fine since we determine the resulting name before-hand unlike other situations
        // where gradle is the one to determine it.
        //
        // and in the case that profile is `Debug` there will be only one path that has the suffix `debug`
        let all_apks_path = Self::all_apks_paths(config, profile, flavor)[0].clone();
        let aab_path = aab::aab_path(config, profile, flavor);
        bundletool::command()
            .before_spawn(move |cmd| {
                cmd.args([
                    "build-apks",
                    &format!("--bundle={}", aab_path.to_str().unwrap()),
                    &format!("--output={}", all_apks_path.to_str().unwrap()),
                    "--connected-device",
                ]);
                Ok(())
            })
            .run()
            .map_err(ApksBuildError::BuildFromAabFailed)?;
        Ok(())
    }

    fn install_apk_from_aab(
        &self,
        config: &Config,
        profile: Profile,
    ) -> Result<(), ApkInstallError> {
        let flavor = self.target.arch;
        let apks_path = Self::all_apks_paths(config, profile, flavor)
            .into_iter()
            .reduce(last_modified)
            .unwrap();
        bundletool::command()
            .before_spawn(move |cmd| {
                cmd.args([
                    "install-apks",
                    &format!("--apks={}", apks_path.to_str().unwrap()),
                ]);

                Ok(())
            })
            .run()
            .map_err(ApkInstallError::InstallFromAabFailed)?;
        Ok(())
    }

    fn wake_screen(&self, env: &Env) -> std::io::Result<()> {
        self.adb(env)
            .before_spawn(move |cmd| {
                cmd.args(["shell", "input", "keyevent", "KEYCODE_WAKEUP"]);
                Ok(())
            })
            .dup_stdio()
            .start()?
            .wait()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
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
    ) -> Result<duct::Handle, RunError> {
        if build_app_bundle {
            bundletool::install(reinstall_deps).map_err(RunError::BundletoolInstallFailed)?;
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
        let activity = format!("{}/{}", config.app().reverse_identifier(), activity);
        self.adb(env)
            .before_spawn(move |cmd| {
                cmd.args(["shell", "am", "start", "-n", &activity]);
                Ok(())
            })
            .dup_stdio()
            .start()?
            .wait()?;

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

        let stdout = loop {
            let cmd = duct::cmd(
                env.platform_tools_path().join("adb"),
                ["shell", "pidof", "-s", &config.app().reverse_identifier()],
            )
            .vars(env.explicit_env())
            .stderr_capture()
            .stdout_capture();
            let handle = cmd.start()?;
            if let Ok(out) = handle.wait() {
                if out.status.success() {
                    break String::from_utf8_lossy(&out.stdout).into_owned();
                }
            }
            sleep(Duration::from_secs(2));
        };
        let pid = stdout.trim().to_string();
        let mut logcat = duct::cmd(
            env.platform_tools_path().join("adb"),
            ["logcat", "-v", "color", "-s", &filter],
        )
        .vars(env.explicit_env())
        .dup_stdio();

        let logcat_filter_specs = config.logcat_filter_specs().to_vec();
        logcat = logcat.before_spawn(move |cmd| {
            if !pid.is_empty() {
                cmd.args(["--pid", &pid]);
            }
            cmd.args(&logcat_filter_specs);
            Ok(())
        });
        logcat.start().map_err(Into::into)
    }

    pub fn stacktrace(&self, config: &Config, env: &Env) -> Result<(), StacktraceError> {
        let jnilib_path = config
            .app()
            // ndk-stack can't seem to handle spaces in args, no matter
            // how I try to quote or escape them... so, instead of
            // mandating that the entire path not contain spaces, we'll
            // just use a relative path!
            .unprefix_path(jnilibs::path(config, *self.target))
            .expect("developer error: jnilibs subdir not prefixed");
        // -d = print and exit
        let logcat_command = adb::adb(env, &self.serial_no)
            .before_spawn(move |cmd| {
                cmd.args(["logcat", "-d"]);
                cmd.arg("-sym");
                cmd.arg(&jnilib_path);
                Ok(())
            })
            .dup_stdio();
        let stack_command =
            duct::cmd::<PathBuf, [String; 0]>(env.ndk.home().join(consts::NDK_STACK), [])
                .vars(env.explicit_env())
                .env(
                    "PATH",
                    util::prepend_to_path(env.ndk.home().display(), env.path().to_string_lossy()),
                )
                .dup_stdio();

        if logcat_command.pipe(stack_command).start()?.wait().is_err() {
            println!("  -- no stacktrace --");
        }
        Ok(())
    }
}
