use super::{config::Config, env::Env, target::Target};
use crate::{
    env::ExplicitEnv,
    open_harmony::{hap, hdc},
    opts::{NoiseLevel, Profile},
    util::{
        cli::{Report, Reportable},
        last_modified,
    },
    DuctExpressionExt,
};
use std::fmt::{self, Display};
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
}

impl Reportable for ApksBuildError {
    fn report(&self) -> Report {
        match self {
            Self::CleanFailed(err) => Report::error("Failed to clean old APKS", err),
        }
    }
}

#[derive(Debug, Error)]
pub enum ApkInstallError {
    #[error("Failed to install APK: {0}")]
    InstallFailed(#[from] std::io::Error),
}

impl Reportable for ApkInstallError {
    fn report(&self) -> Report {
        match self {
            Self::InstallFailed(err) => Report::error("Failed to install APK", err),
        }
    }
}

#[derive(Debug, Error)]
pub enum RunError {
    #[error(transparent)]
    HapError(hap::HapError),
    #[error(transparent)]
    ApkInstallFailed(ApkInstallError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::HapError(err) => err.report(),
            Self::ApkInstallFailed(err) => err.report(),
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
    id: String,
    name: String,
    model: String,
    target: &'a Target<'a>,
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
    pub(super) fn new(id: String, name: String, model: String, target: &'a Target<'a>) -> Self {
        Self {
            id,
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

    pub fn id(&self) -> &str {
        &self.id
    }

    fn hdc(&self, env: &Env) -> duct::Expression {
        hdc::hdc(env, ["-t", &self.id])
    }

    fn wait_device_boot(&self, env: &Env) {
        loop {
            let cmd = self
                .hdc(env)
                .stderr_capture()
                .stdout_capture()
                .before_spawn(move |cmd| {
                    cmd.args(["shell", "param", "get", "ohos.boot.time.init"]);
                    Ok(())
                });
            let handle = cmd.start();
            if let Ok(handle) = handle {
                if let Ok(output) = handle.wait() {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if stdout.trim().parse::<usize>().is_ok() {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                } else {
                    break;
                }
            }
        }
    }

    fn build_hap(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<(), hap::HapError> {
        hap::build(config, env, noise_level, profile)?;
        Ok(())
    }

    fn install_hap(&self, config: &Config, env: &Env) -> Result<(), std::io::Error> {
        let hap_path = hap::haps_paths(config)
            .into_iter()
            .reduce(last_modified)
            .unwrap();

        self.hdc(env)
            .before_spawn(move |cmd| {
                cmd.args(["install", "-r"]);
                cmd.arg(&hap_path);
                Ok(())
            })
            .dup_stdio()
            .start()?
            .wait()?;

        Ok(())
    }

    fn wake_screen(&self, _env: &Env) -> std::io::Result<()> {
        // TODO: seems like there's no equivalent to `adb shell input keyevent KEYCODE_WAKEUP`
        // in hdc (a keyevent can be sent with `hdc shell uitest uiInput keyEvent Power`)
        Ok(())
    }

    // see https://developer.huawei.com/consumer/en/doc/harmonyos-guides/web-debugging-with-devtools
    fn setup_devtools_port_forwarding_async(&self, env: &Env, pid: &str) {
        let explicit_env = env.explicit_env();
        let hdc_path = env.toolchains_path().join("hdc");
        let pid = pid.to_string();

        std::thread::spawn(move || {
            const MAX_ATTEMPTS: usize = 10;
            let mut retries = 0;

            // wait for the remote devtools socket to be opened
            let expected_open_socket_content = format!("@webview_devtools_remote_{pid}");
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let Ok(opened_sockets) = duct::cmd(&hdc_path, ["shell", "cat", "/proc/net/unix"])
                    .vars(explicit_env.clone())
                    .read()
                else {
                    break;
                };
                if opened_sockets.contains(&expected_open_socket_content) {
                    break;
                }

                retries += 1;
                if retries >= MAX_ATTEMPTS {
                    log::error!(
                        "Could not setup port forwarding for devtools. Make sure you are running setWebDebuggingAccess(true). See https://developer.huawei.com/consumer/en/doc/harmonyos-guides/web-debugging-with-devtools for more information."
                    );
                    return;
                }
            }

            // forward the remote devtools socket to the local port 9222
            // so Chrome can connect to it
            let _ = duct::cmd(
                &hdc_path,
                [
                    "fport",
                    "tcp:9222",
                    &format!("localabstract:webview_devtools_remote_{pid}"),
                ],
            )
            .vars(explicit_env)
            .dup_stdio()
            .start();
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<duct::Handle, RunError> {
        self.build_hap(config, env, noise_level, profile)
            .map_err(RunError::HapError)?;
        if self.model.starts_with("emulator") {
            self.wait_device_boot(env);
        }
        self.install_hap(config, env).map_err(RunError::Io)?;

        let identifier = config.app().identifier().to_string();
        self.hdc(env)
            .before_spawn(move |cmd| {
                cmd.args([
                    "shell",
                    "aa",
                    "start",
                    "-b",
                    &identifier,
                    "-a",
                    "EntryAbility",
                ]);
                Ok(())
            })
            .dup_stdio()
            .start()?
            .wait()?;

        let _ = self.wake_screen(env);

        let stdout = loop {
            let cmd = duct::cmd(
                env.toolchains_path().join("hdc"),
                ["shell", "pidof", "-s", config.app().identifier()],
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
            std::thread::sleep(std::time::Duration::from_secs(2));
        };
        let pid = stdout.trim().to_string();

        self.setup_devtools_port_forwarding_async(env, &pid);

        let mut logcat = duct::cmd(env.toolchains_path().join("hdc"), ["hilog", "-v", "color"])
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
        let lib_path = config
            .project_dir()
            .join("libs")
            .join(self.target.arch)
            .join(config.app().lib_name())
            .with_extension("so");

        // -x = print and exit
        let hilog_command = hdc::hdc(env, ["-t", &self.id])
            .before_spawn(move |cmd| {
                cmd.args(["hilog", "-x"]);
                cmd.arg("-sym");
                cmd.arg(&lib_path);
                Ok(())
            })
            .dup_stdio();

        if hilog_command.start()?.wait().is_err() {
            println!("  -- no stacktrace --");
        }
        Ok(())
    }
}
