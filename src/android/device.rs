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
pub enum ApksBuildError {
    #[error("Failed to build APKS from AAB: {0}")]
    BuildFromAabFailed(std::io::Error),
}

impl Reportable for ApksBuildError {
    fn report(&self) -> Report {
        match self {
            Self::BuildFromAabFailed(err) => Report::error("Failed to build APKS from AAB", err),
        }
    }
}

#[derive(Debug, Error)]
pub enum ApkInstallError {
    #[error("failed to install APK with {command}: {error}")]
    InstallFailed {
        command: String,
        error: std::io::Error,
    },
    #[error("failed to install APK from AAB with {command}: {error}")]
    InstallFromAabFailed {
        command: String,
        error: std::io::Error,
    },
}

impl Reportable for ApkInstallError {
    fn report(&self) -> Report {
        match self {
            Self::InstallFailed { command, error } => {
                Report::error(format!("Failed to install APK with {command}"), error)
            }
            Self::InstallFromAabFailed { command, error } => Report::error(
                format!("Failed to install APK from AAB with {command}"),
                error,
            ),
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
    ApksFromAabBuildFailed(ApksBuildError),
    #[error("failed to run {command}: {error}")]
    CommandFailed {
        command: String,
        error: std::io::Error,
    },
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::ApkError(err) => err.report(),
            Self::AabError(err) => err.report(),
            Self::ApkInstallFailed(err) => err.report(),
            Self::WakeScreenFailed(err) => Report::error("Failed to wake device screen", err),
            Self::BundletoolInstallFailed(err) => err.report(),
            Self::ApksFromAabBuildFailed(err) => err.report(),
            Self::CommandFailed { command, error } => {
                Report::error(format!("Command failed with {command}"), error)
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StacktraceError {
    #[error("failed to run {command}: {error}")]
    CommandFailed {
        command: String,
        error: std::io::Error,
    },
}

impl Reportable for StacktraceError {
    fn report(&self) -> Report {
        match self {
            Self::CommandFailed { command, error } => {
                Report::error(format!("Failed to run {command}"), error)
            }
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
#[non_exhaustive]
pub enum ConnectionStatus {
    Connected,
    Offline,
    Unauthorized,
    Authorizing,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Device<'a> {
    serial_no: String,
    name: String,
    model: String,
    target: &'a Target<'a>,
    status: ConnectionStatus,
}

impl Display for Device<'_> {
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
        status: ConnectionStatus,
    ) -> Self {
        Self {
            serial_no,
            name,
            model,
            target,
            status,
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

    pub fn serial_no(&self) -> &str {
        &self.serial_no
    }

    pub fn status(&self) -> ConnectionStatus {
        self.status
    }

    fn adb(&self, env: &Env) -> duct::Expression {
        adb::adb(env, ["-s", &self.serial_no])
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
                if let Ok(Some(output)) = handle.wait_timeout(Duration::from_secs(3)) {
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

        let cmd = self
            .adb(env)
            .before_spawn(move |cmd| {
                cmd.args(["install", "-r"]);
                cmd.arg(&apk_path);
                Ok(())
            })
            .dup_stdio();
        cmd.start()
            .map_err(|err| ApkInstallError::InstallFailed {
                command: format!("{cmd:?}"),
                error: err,
            })?
            .wait()
            .map_err(|err| ApkInstallError::InstallFailed {
                command: format!("{cmd:?}"),
                error: err,
            })?;

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
        let cmd = bundletool::command();
        let cmd_str = format!(
            "{cmd:?} install-apks --apks={}",
            apks_path.to_str().unwrap()
        );
        let cmd = cmd.before_spawn(move |cmd| {
            cmd.args([
                "install-apks",
                &format!("--apks={}", apks_path.to_str().unwrap()),
            ]);

            Ok(())
        });
        cmd.run()
            .map_err(|err| ApkInstallError::InstallFromAabFailed {
                command: cmd_str,
                error: err,
            })?;
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
            .wait_timeout(Duration::from_secs(3))
            .and_then(|output| {
                output.ok_or(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "adb shell input keyevent KEYCODE_WAKEUP timed out",
                ))
            })?;
        Ok(())
    }

    /**
     * This is the legacy function that doesn't support applicationIdSuffix, which
     * is required for running different build variants (such as debug and release).
     *
     * It is kept for backwards compatibility.
     */
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
        return self.run_with_application_id_suffix(
            config,
            env,
            noise_level,
            profile,
            filter_level,
            build_app_bundle,
            reinstall_deps,
            activity,
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_with_application_id_suffix(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
        filter_level: Option<FilterLevel>,
        build_app_bundle: bool,
        reinstall_deps: bool,
        activity: String,
        application_id_suffix: Option<String>,
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

        let app_identifier = Device::resolve_application_id(
            config.app().identifier(),
            application_id_suffix.as_deref(),
        );
        let activity = Device::resolve_activity_name(app_identifier.clone(), activity);
        let activity_ = activity.clone();
        let cmd = self
            .adb(env)
            .before_spawn(move |cmd| {
                cmd.args(["shell", "am", "start", "-n", &activity_]);
                Ok(())
            })
            .dup_stdio();
        cmd.start()
            .map_err(|err| RunError::CommandFailed {
                command: format!("{cmd:?} shell am start -n {activity}"),
                error: err,
            })?
            .wait()
            .map_err(|err| RunError::CommandFailed {
                command: format!("{cmd:?} shell am start -n {activity}"),
                error: err,
            })?;

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
                ["shell", "pidof", "-s", app_identifier.as_str()],
            )
            .vars(env.explicit_env())
            .stderr_capture()
            .stdout_capture();
            let handle = cmd.start().map_err(|err| RunError::CommandFailed {
                command: format!("{cmd:?}"),
                error: err,
            })?;
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
        logcat.start().map_err(|err| RunError::CommandFailed {
            command: format!("{logcat:?}"),
            error: err,
        })
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
        let logcat_command = adb::adb(env, ["-s", &self.serial_no])
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

        if logcat_command
            .pipe(&stack_command)
            .start()
            .map_err(|err| StacktraceError::CommandFailed {
                command: format!("{logcat_command:?} | {stack_command:?}"),
                error: err,
            })?
            .wait()
            .is_err()
        {
            println!("  -- no stacktrace --");
        }
        Ok(())
    }

    /**
     * The application ID is the package name of the variant to launch.
     */
    fn resolve_application_id(app_identifier: &str, application_id_suffix: Option<&str>) -> String {
        format!(
            "{app_identifier}{}",
            application_id_suffix.unwrap_or_default()
        )
    }

    /**
     * The activity name is the fully qualified class name of the activity to launch.
     * Here are the expected formats for the activity name: of the release and debug variants
     *
     * Even though the actual logic of this method is very simple, it is kept
     * separate for clarity and for unit testing.
     *
     * Release variants have 2 acceptable formats:
     * * `com.example.app/.MainActivity`
     * * `com.example.app/com.example.app.MainActivity`
     *
     * Debug variants have 1 acceptable format:
     * * `com.example.app.debug/com.example.app.MainActivity`
     */
    fn resolve_activity_name(app_identifier: String, activity: String) -> String {
        format!("{app_identifier}/{activity}")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;
    #[rstest(
        app_identifier,
        application_id_suffix,
        activity,
        expected,
        case(
            "com.example.app",
            Some(".debug"),
            "com.example.app.MainActivity",
            "com.example.app.debug/com.example.app.MainActivity"
        ),
        case(
            "com.example.app",
            None,
            ".MainActivity",
            "com.example.app/.MainActivity"
        ),
        case(
            "com.example.app",
            None,
            "com.example.app.MainActivity",
            "com.example.app/com.example.app.MainActivity"
        )
    )]
    fn test_resolve_activity_name(
        app_identifier: String,
        application_id_suffix: Option<&str>,
        activity: String,
        expected: String,
    ) {
        let app_identifier = Device::resolve_application_id(&app_identifier, application_id_suffix);
        let activity = Device::resolve_activity_name(app_identifier, activity);
        assert_eq!(activity, expected);
    }
}
