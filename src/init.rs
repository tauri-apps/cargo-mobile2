#[cfg(feature = "android")]
use crate::android;
#[cfg(feature = "apple")]
use crate::apple;
use crate::{
    config::{self, Config},
    dot_cargo, opts, project,
    steps::{self, Steps},
    templating,
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

pub static DOT_FIRST_INIT_FILE_NAME: &'static str = ".first-init";
static DOT_FIRST_INIT_CONTENTS: &'static str = // newline
r#"The presence of this file indicates `cargo mobile init` has been called for
the first time on a new project, but hasn't yet completed successfully once. As
long as this file is here, `cargo mobile init` will use a more aggressive
template generation strategy that allows it to place files that it wouldn't
normally be able to modify.

If you believe this file isn't supposed to be here, please report this, and then
delete this file to regain normal behavior.

Alternatively, if you do that and then realize that you were wrong (ouch!) and
your project was never fully generated, then you can just create `.first-init`
again (the contents don't matter) and run `cargo mobile init`. Just, if you do
that, any generated files you modified will be overwritten!
"#;

#[derive(Debug)]
pub enum Error {
    ConfigLoadOrGenFailed(config::LoadOrGenError),
    DotFirstInitWriteFailed {
        path: PathBuf,
        cause: io::Error,
    },
    FilterConfigureFailed(templating::FilterError),
    OnlyParseFailed(steps::NotRegistered),
    SkipParseFailed(steps::NotRegistered),
    ProjectInitFailed(project::Error),
    AssetDirCreationFailed {
        asset_dir: PathBuf,
        cause: io::Error,
    },
    CodeCommandPresentFailed(bossy::Error),
    LldbExtensionInstallFailed(bossy::Error),
    DotCargoLoadFailed(dot_cargo::LoadError),
    HostTargetTripleDetectionFailed(util::HostTargetTripleError),
    #[cfg(feature = "android")]
    AndroidEnvFailed(android::env::Error),
    #[cfg(feature = "android")]
    AndroidInitFailed(android::project::Error),
    #[cfg(feature = "apple")]
    AppleInitFailed(apple::project::Error),
    DotCargoWriteFailed(dot_cargo::WriteError),
    DotFirstInitDeleteFailed {
        path: PathBuf,
        cause: io::Error,
    },
    OpenInEditorFailed(util::OpenInEditorError),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::ConfigLoadOrGenFailed(err) => err.report(),
            Self::DotFirstInitWriteFailed { path, cause } => Report::error(format!("Failed to write first init dot file {:?}", path), cause),
            Self::FilterConfigureFailed(err) => Report::error("Failed to configure template filter", err),
            Self::OnlyParseFailed(err) => Report::error("Failed to parse `only` step list", err),
            Self::SkipParseFailed(err) => Report::error("Failed to parse `skip` step list", err),
            Self::ProjectInitFailed(err) => err.report(),
            Self::AssetDirCreationFailed { asset_dir, cause } => Report::error(format!("Failed to create asset dir {:?}", asset_dir), cause),
            Self::CodeCommandPresentFailed(err) => Report::error("Failed to check for presence of `code` command", err),
            Self::LldbExtensionInstallFailed(err) => Report::error("Failed to install CodeLLDB extension", err),
            Self::DotCargoLoadFailed(err) => err.report(),
            Self::HostTargetTripleDetectionFailed(err) => err.report(),
            #[cfg(feature = "android")]
            Self::AndroidEnvFailed(err) => err.report(),
            #[cfg(feature = "android")]
            Self::AndroidInitFailed(err) => err.report(),
            #[cfg(feature = "apple")]
            Self::AppleInitFailed(err) => err.report(),
            Self::DotCargoWriteFailed(err) => err.report(),
            Self::DotFirstInitDeleteFailed { path, cause } => Report::action_request(format!("Failed to delete first init dot file {:?}; the project generated successfully, but `cargo mobile init` will have unexpected results unless you manually delete this file!", path), cause),
            Self::OpenInEditorFailed(err) => Report::error("Failed to open project in editor (your project generated successfully though, so no worries!)", err),
        }
    }
}

pub fn exec(
    wrapper: &TextWrapper,
    non_interactive: opts::NonInteractive,
    skip_dev_tools: opts::SkipDevTools,
    reinstall_deps: opts::ReinstallDeps,
    open_in_editor: opts::OpenInEditor,
    only: Option<Vec<String>>,
    skip: Option<Vec<String>>,
    cwd: impl AsRef<Path>,
) -> Result<Config, Error> {
    let cwd = cwd.as_ref();
    let (config, config_origin) =
        Config::load_or_gen(cwd, non_interactive, wrapper).map_err(Error::ConfigLoadOrGenFailed)?;
    let dot_first_init_path = config.app().root_dir().join(DOT_FIRST_INIT_FILE_NAME);
    let dot_first_init_exists = dot_first_init_path.exists();
    if config_origin.freshly_minted() && !dot_first_init_exists {
        // indicate first init is ongoing, so that if we error out and exit
        // the next init will know to still use `WildWest` filtering
        log::info!("creating first init dot file at {:?}", dot_first_init_path);
        fs::write(&dot_first_init_path, DOT_FIRST_INIT_CONTENTS).map_err(|cause| {
            Error::DotFirstInitWriteFailed {
                path: dot_first_init_path.clone(),
                cause,
            }
        })?;
    }
    let bike = config.build_a_bike();
    let filter = templating::Filter::new(&config, config_origin, dot_first_init_exists)
        .map_err(Error::FilterConfigureFailed)?;
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
        project::gen(&config, &bike, &filter).map_err(Error::ProjectInitFailed)?;
    }
    let asset_dir = config.app().asset_dir();
    if !asset_dir.is_dir() {
        fs::create_dir_all(&asset_dir)
            .map_err(|cause| Error::AssetDirCreationFailed { asset_dir, cause })?;
    }
    if skip_dev_tools.no()
        && util::command_present("code").map_err(Error::CodeCommandPresentFailed)?
    {
        let mut command = bossy::Command::impure("code")
            .with_args(&["--install-extension", "vadimcn.vscode-lldb"]);
        if non_interactive.yes() {
            command.add_arg("--force");
        }
        command
            .run_and_wait()
            .map_err(Error::LldbExtensionInstallFailed)?;
    }
    let mut dot_cargo =
        dot_cargo::DotCargo::load(config.app()).map_err(Error::DotCargoLoadFailed)?;
    // Mysteriously, builds that don't specify `--target` seem to fight over
    // the build cache with builds that use `--target`! This means that
    // alternating between i.e. `cargo run` and `cargo apple run` would
    // result in clean builds being made each time you switched... which is
    // pretty nightmarish. Specifying `build.target` in `.cargo/config`
    // fortunately has the same effect as specifying `--target`, so now we can
    // `cargo run` with peace of mind!
    //
    // This behavior could be explained here:
    // https://doc.rust-lang.org/cargo/reference/config.html#buildrustflags
    dot_cargo.set_default_target(
        util::host_target_triple().map_err(Error::HostTargetTripleDetectionFailed)?,
    );
    #[cfg(feature = "android")]
    {
        if steps.is_set("android") {
            let env = android::env::Env::new().map_err(Error::AndroidEnvFailed)?;
            android::project::gen(config.android(), &env, &bike, &filter, &mut dot_cargo)
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
                skip_dev_tools,
                reinstall_deps,
                &filter,
            )
            .map_err(Error::AppleInitFailed)?;
        }
    }
    dot_cargo
        .write(config.app())
        .map_err(Error::DotCargoWriteFailed)?;
    if dot_first_init_exists {
        log::info!("deleting first init dot file at {:?}", dot_first_init_path);
        fs::remove_file(&dot_first_init_path).map_err(|cause| Error::DotFirstInitDeleteFailed {
            path: dot_first_init_path,
            cause,
        })?;
    }
    if open_in_editor.yes() {
        util::open_in_editor(cwd).map_err(Error::OpenInEditorFailed)?;
    }
    Ok(config)
}
