use self::{cargo::CargoConfig, steps::Steps};
use crate::{
    config::Config,
    opts::{Clobbering, OpenIn},
    target::TargetTrait as _,
    util::{
        self,
        prompt::{self, YesOrNo},
    },
};
use into_result::{command::CommandError, IntoResult as _};
use std::{fmt, io, path::Path, process::Command};

#[derive(Debug)]
pub enum Error {
    MigrationPromptFailed(io::Error),
    MigrationFailed(migrate::Error),
    CargoConfigGenFailed(cargo::GenError),
    CargoConfigWriteFailed(cargo::WriteError),
    // HelloWorldGenFailed(rust::Error),
    // AndroidRustupFailed(CommandError),
    // AndroidGenFailed(android::project::Error),
    // IosDepsFailed(IosDepsError),
    // IosRustupFailed(CommandError),
    // IosGenFailed(ios::project::Error),
    PluginFailed(Box<dyn fmt::Debug + fmt::Display>),
    OpenInEditorFailed(CommandError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MigrationPromptFailed(err) => {
                write!(f, "Failed to prompt for migration: {}", err)
            }
            Error::MigrationFailed(err) => write!(
                f,
                "Failed to migrate project - project state is now undefined! 💀: {}",
                err
            ),
            Error::CargoConfigGenFailed(err) => {
                write!(f, "Failed to generate \".cargo/config\": {}", err)
            }
            Error::CargoConfigWriteFailed(err) => {
                write!(f, "Failed to write \".cargo/config\": {}", err)
            }
            // Error::HelloWorldGenFailed(err) => {
            //     write!(f, "Failed to generate hello world project: {}", err)
            // }
            // Error::AndroidRustupFailed(err) => {
            //     write!(f, "Failed to `rustup` Android toolchains: {}", err)
            // }
            // Error::AndroidGenFailed(err) => {
            //     write!(f, "Failed to generate Android project: {}", err)
            // }
            // Error::IosDepsFailed(err) => write!(f, "Failed to install iOS dependencies: {}", err),
            // Error::IosRustupFailed(err) => write!(f, "Failed to `rustup` iOS toolchains: {}", err),
            // Error::IosGenFailed(err) => write!(f, "Failed to generate iOS project: {}", err),
            Error::OpenInEditorFailed(err) => write!(f, "Failed to open project in editor (your project generated successfully though, so no worries): {}", err),
        }
    }
}

// TODO: Don't redo things if no changes need to be made
pub fn init(
    config: &Config,
    bike: &bicycle::Bicycle,
    clobbering: Clobbering,
    open: OpenIn,
    only: Option<impl Into<Steps>>,
    skip: Option<impl Into<Steps>>,
) -> Result<(), Error> {
    if let Some(proj) = migrate::LegacyProject::heuristic_detect(config) {
        println!(
            r#"
    It looks like you're using the old project structure, which is now unsupported.
    The new project structure is super sleek, and ginit can migrate your project
    automatically! However, this can potentially fail. Be sure you have a backup of
    your project in case things explode. You've been warned! 💀
            "#
        );
        let response = prompt::yes_no(
            "I have a backup, and I'm ready to migrate",
            Some(YesOrNo::No),
        )
        .map_err(Error::MigrationPromptFailed)?;
        match response {
            Some(YesOrNo::Yes) => {
                proj.migrate(config).map_err(Error::MigrationFailed)?;
                println!("Migration successful! 🎉\n");
            }
            Some(YesOrNo::No) => {
                println!("Maybe next time. Buh-bye!");
                return Ok(());
            }
            None => {
                println!("That was neither a Y nor an N! You're pretty silly.");
                return Ok(());
            }
        }
    }
    let steps = {
        let only = only.map(Into::into).unwrap_or_else(|| Steps::all());
        let skip = skip.map(Into::into).unwrap_or_else(|| Steps::empty());
        only & !skip
    };
    if steps.contains(Steps::CARGO) {
        CargoConfig::generate(config, &steps)
            .map_err(Error::CargoConfigGenFailed)?
            .write(&config)
            .map_err(Error::CargoConfigWriteFailed)?;
    }
    // if steps.contains(Steps::HELLO_WORLD) {
    //     rust::hello_world(config, bike, clobbering).map_err(Error::HelloWorldGenFailed)?;
    // }
    if steps.contains(Steps::ANDROID) {
        if steps.contains(Steps::TOOLCHAINS) {
            for target in android::target::Target::all().values() {
                target.rustup_add().map_err(Error::AndroidRustupFailed)?;
            }
        }
        android::project::create(config, bike).map_err(Error::AndroidGenFailed)?;
    }
    // if steps.contains(Steps::IOS) {
    //     if steps.contains(Steps::DEPS) {
    //         install_ios_deps(clobbering).map_err(Error::IosDepsFailed)?;
    //     }
    //     if steps.contains(Steps::TOOLCHAINS) {
    //         for target in ios::target::Target::all().values() {
    //             target.rustup_add().map_err(Error::IosRustupFailed)?;
    //         }
    //     }
    //     ios::project::create(config, bike).map_err(Error::IosGenFailed)?;
    // }
    if let OpenIn::Editor = open {
        util::open_in_editor(".").map_err(Error::OpenInEditorFailed)?;
    }
    Ok(())
}
