use crate::android;
#[cfg(target_os = "macos")]
use crate::apple;
use crate::{
    config::{
        self,
        metadata::{self, Metadata},
        Config,
    },
    dot_cargo,
    os::code_command,
    project, templating,
    util::{
        self,
        cli::{Report, Reportable, TextWrapper},
    },
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub static DOT_FIRST_INIT_FILE_NAME: &str = ".first-init";
static DOT_FIRST_INIT_CONTENTS: &str = // newline
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
    ProjectInitFailed(project::Error),
    AssetDirCreationFailed {
        asset_dir: PathBuf,
        cause: io::Error,
    },
    CodeCommandPresentFailed(std::io::Error),
    LldbExtensionInstallFailed(std::io::Error),
    DotCargoLoadFailed(dot_cargo::LoadError),
    HostTargetTripleDetectionFailed(util::HostTargetTripleError),
    MetadataFailed(metadata::Error),
    #[cfg(target_os = "macos")]
    AppleInitFailed(apple::project::Error),
    AndroidEnvFailed(android::env::Error),
    AndroidInitFailed(android::project::Error),
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
            Self::ProjectInitFailed(err) => err.report(),
            Self::AssetDirCreationFailed { asset_dir, cause } => Report::error(format!("Failed to create asset dir {:?}", asset_dir), cause),
            Self::CodeCommandPresentFailed(err) => Report::error("Failed to check for presence of `code` command", err),
            Self::LldbExtensionInstallFailed(err) => Report::error("Failed to install CodeLLDB extension", err),
            Self::DotCargoLoadFailed(err) => err.report(),
            Self::HostTargetTripleDetectionFailed(err) => err.report(),
            Self::MetadataFailed(err) => err.report(),
            Self::AndroidEnvFailed(err) => err.report(),
            Self::AndroidInitFailed(err) => err.report(),
            #[cfg(target_os = "macos")]
            Self::AppleInitFailed(err) => err.report(),
            Self::DotCargoWriteFailed(err) => err.report(),
            Self::DotFirstInitDeleteFailed { path, cause } => Report::action_request(format!("Failed to delete first init dot file {:?}; the project generated successfully, but `cargo mobile init` will have unexpected results unless you manually delete this file!", path), cause),
            Self::OpenInEditorFailed(err) => Report::error("Failed to open project in editor (your project generated successfully though, so no worries!)", err),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn exec(
    wrapper: &TextWrapper,
    non_interactive: bool,
    skip_dev_tools: bool,
    skip_targets_install: bool,
    #[cfg_attr(not(target_os = "macos"), allow(unused))] reinstall_deps: bool,
    open_in_editor: bool,
    submodule_commit: Option<String>,
    cwd: impl AsRef<Path>,
) -> Result<Config, Box<Error>> {
    let cwd = cwd.as_ref();
    let (config, config_origin) =
        Config::load_or_gen(cwd, non_interactive, wrapper).map_err(Error::ConfigLoadOrGenFailed)?;
    let dot_first_init_path = config.app().root_dir().join(DOT_FIRST_INIT_FILE_NAME);
    let dot_first_init_exists = {
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
            true
        } else {
            dot_first_init_exists
        }
    };
    let bike = config.build_a_bike();
    let filter = templating::Filter::new(&config, config_origin, dot_first_init_exists)
        .map_err(Error::FilterConfigureFailed)?;

    // Generate the base project
    project::gen(&config, &bike, &filter, submodule_commit).map_err(Error::ProjectInitFailed)?;

    let asset_dir = config.app().asset_dir();
    if !asset_dir.is_dir() {
        fs::create_dir_all(&asset_dir)
            .map_err(|cause| Error::AssetDirCreationFailed { asset_dir, cause })?;
    }
    if !skip_dev_tools && util::command_present("code").map_err(Error::CodeCommandPresentFailed)? {
        code_command()
            .before_spawn(move |cmd| {
                cmd.args(["--install-extension", "vadimcn.vscode-lldb"]);
                if non_interactive {
                    cmd.arg("--force");
                }
                Ok(())
            })
            .run()
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

    let metadata = Metadata::load(config.app().root_dir()).map_err(Error::MetadataFailed)?;

    // Generate Xcode project
    #[cfg(target_os = "macos")]
    if metadata.apple().supported() {
        apple::project::gen(
            config.apple(),
            metadata.apple(),
            config.app().template_pack().submodule_path(),
            &bike,
            wrapper,
            non_interactive,
            skip_dev_tools,
            reinstall_deps,
            &filter,
            skip_targets_install,
        )
        .map_err(Error::AppleInitFailed)?;
    } else {
        println!("Skipping iOS init, since it's marked as unsupported in your Cargo.toml metadata");
    }

    // Generate Android Studio project
    if metadata.android().supported() {
        match android::env::Env::new() {
            Ok(env) => android::project::gen(
                config.android(),
                metadata.android(),
                &env,
                &bike,
                wrapper,
                &filter,
                &mut dot_cargo,
                skip_targets_install,
            )
            .map_err(Error::AndroidInitFailed)?,
            Err(err) => {
                if err.sdk_or_ndk_issue() {
                    Report::action_request(
                        "Failed to initialize Android environment; Android support won't be usable until you fix the issue below and re-run `cargo mobile init`!",
                        err,
                    )
                    .print(wrapper);
                } else {
                    Err(Error::AndroidEnvFailed(err))?;
                }
            }
        }
    } else {
        println!(
            "Skipping Android init, since it's marked as unsupported in your Cargo.toml metadata"
        );
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
    Report::victory(
        "Project generated successfully!",
        "Make cool apps! 🌻 🐕 🎉",
    )
    .print(wrapper);
    if open_in_editor {
        util::open_in_editor(cwd).map_err(Error::OpenInEditorFailed)?;
    }
    Ok(config)
}
