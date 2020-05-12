#[cfg(feature = "android")]
use crate::android;
#[cfg(feature = "apple")]
use crate::apple;
use crate::{
    config::{self, Config},
    opts, project,
    steps::{self, Steps},
    util::{
        self,
        cli::{Report, Reportable, TextWrapper},
    },
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub static STEPS: &'static [&'static str] = &[
    "project",
    #[cfg(feature = "android")]
    "android",
    #[cfg(feature = "apple")]
    "apple",
];

#[derive(Debug)]
pub enum Error {
    ConfigLoadOrGenFailed(config::LoadOrGenError),
    OnlyParseFailed(steps::NotRegistered),
    SkipParseFailed(steps::NotRegistered),
    ProjectInitFailed(project::Error),
    AssetDirCreationFailed {
        asset_dir: PathBuf,
        cause: io::Error,
    },
    CodeCommandPresentFailed(bossy::Error),
    LldbExtensionInstallFailed(bossy::Error),
    #[cfg(feature = "android")]
    AndroidEnvFailed(android::env::Error),
    #[cfg(feature = "android")]
    AndroidInitFailed(android::project::Error),
    #[cfg(feature = "apple")]
    AppleInitFailed(apple::project::Error),
    OpenInEditorFailed(util::OpenInEditorError),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::ConfigLoadOrGenFailed(err) => err.report(),
            Self::OnlyParseFailed(err) => Report::error("Failed to parse `only` step list", err),
            Self::SkipParseFailed(err) => Report::error("Failed to parse `skip` step list", err),
            Self::ProjectInitFailed(err) => err.report(),
            Self::AssetDirCreationFailed { asset_dir, cause } => Report::error(format!("Failed to create asset dir {:?}", asset_dir), cause),
            Self::CodeCommandPresentFailed(err) => Report::error("Failed to check for presence of `code` command", err),
            Self::LldbExtensionInstallFailed(err) => Report::error("Failed to install CodeLLDB extension", err),
            #[cfg(feature = "android")]
            Self::AndroidEnvFailed(err) => err.report(),
            #[cfg(feature = "android")]
            Self::AndroidInitFailed(err) => err.report(),
            #[cfg(feature = "apple")]
            Self::AppleInitFailed(err) => err.report(),
            Self::OpenInEditorFailed(err) => Report::error("Failed to open project in editor (your project generated successfully though, so no worries!)", err),
        }
    }
}

pub fn exec(
    wrapper: &TextWrapper,
    interactivity: opts::Interactivity,
    clobbering: opts::Clobbering,
    open_in: opts::OpenIn,
    only: Option<Vec<String>>,
    skip: Option<Vec<String>>,
    cwd: impl AsRef<Path>,
) -> Result<Config, Error> {
    let cwd = cwd.as_ref();
    let config =
        Config::load_or_gen(cwd, interactivity, wrapper).map_err(Error::ConfigLoadOrGenFailed)?;
    let bike = config.build_a_bike();
    let step_registry = steps::Registry::new(STEPS);
    let steps = {
        let only = only
            .as_ref()
            .map(|only| Steps::parse(&step_registry, only.as_slice()))
            .unwrap_or_else(|| Ok(Steps::new_all_set(&step_registry)))
            .map_err(Error::OnlyParseFailed)?;
        let skip = skip
            .as_ref()
            .map(|skip| Steps::parse(&step_registry, skip.as_slice()))
            .unwrap_or_else(|| Ok(Steps::new_all_unset(&step_registry)))
            .map_err(Error::SkipParseFailed)?;
        Steps::from_bits(&step_registry, only.bits() & !skip.bits())
    };
    if steps.is_set("project") {
        project::gen(&config, &bike, clobbering).map_err(Error::ProjectInitFailed)?;
        let asset_dir = config.app().asset_dir();
        if !asset_dir.is_dir() {
            fs::create_dir_all(&asset_dir)
                .map_err(|cause| Error::AssetDirCreationFailed { asset_dir, cause })?;
        }
        if util::command_present("code").map_err(Error::CodeCommandPresentFailed)? {
            let mut command = bossy::Command::impure("code")
                .with_args(&["--install-extension", "vadimcn.vscode-lldb"]);
            if interactivity.none() {
                command.add_arg("--force");
            }
            command
                .run_and_wait()
                .map_err(Error::LldbExtensionInstallFailed)?;
        }
    }
    #[cfg(feature = "android")]
    {
        if steps.is_set("android") {
            let env = android::env::Env::new().map_err(Error::AndroidEnvFailed)?;
            android::project::gen(config.android(), &env, &bike, clobbering)
                .map_err(Error::AndroidInitFailed)?;
        }
    }
    #[cfg(feature = "apple")]
    {
        if steps.is_set("apple") {
            apple::project::gen(
                config.apple(),
                config.app().template_pack().submodule_path(),
                &bike,
                wrapper,
                interactivity,
                clobbering,
            )
            .map_err(Error::AppleInitFailed)?;
        }
    }
    if open_in.editor() {
        util::open_in_editor(cwd).map_err(Error::OpenInEditorFailed)?;
    }
    Ok(config)
}
