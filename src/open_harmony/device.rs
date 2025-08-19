use super::{config::Config, env::Env, target::Target};
use crate::{
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
                    cmd.args(["shell", "getprop", "ohos.boot.time.init"]);
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
        hap::build(config, env, noise_level, profile, vec![self.target()], true)?;
        Ok(())
    }

    fn install_hap(
        &self,
        config: &Config,
        env: &Env,
        profile: Profile,
    ) -> Result<(), std::io::Error> {
        let flavor = self.target.arch;
        let apk_path = hap::haps_paths(config, profile, flavor)
            .into_iter()
            .reduce(last_modified)
            .unwrap();

        self.hdc(env)
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
        self.install_hap(config, env, profile)
            .map_err(RunError::Io)?;
        todo!()
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
