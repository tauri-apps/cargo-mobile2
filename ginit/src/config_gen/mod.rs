mod writer;

use crate::plugin::{Map as PluginMap, RunError};
use ginit_core::{
    config::{shared, DetectedConfigTrait as _, RawConfigTrait as _},
    exports::into_result::command::CommandError,
    opts, util,
};
use std::{
    fmt::{self, Display},
    path::Path,
};

#[derive(Debug)]
pub enum Error {
    SharedDetectionFailed(shared::DetectError),
    SharedPromptFailed(shared::PromptError),
    SharedUpgradeFailed(shared::UpgradeError),
    LoadFailed(writer::LoadError),
    GenFailed {
        plugin_name: String,
        cause: RunError,
    },
    WriteFailed(writer::LinkAndWriteError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SharedDetectionFailed(err) => {
                write!(f, "Failed to detect shared config: {}", err)
            }
            Self::SharedPromptFailed(err) => {
                write!(f, "Failed to prompt for shared config: {}", err)
            }
            Self::SharedUpgradeFailed(err) => {
                write!(f, "Failed to upgrade detected shared config: {}", err)
            }
            Self::LoadFailed(err) => write!(f, "Failed to load existing config: {}", err),
            Self::GenFailed { plugin_name, cause } => write!(
                f,
                "Failed to run `config-gen` command for plugin {:?}: {}",
                plugin_name, cause
            ),
            Self::WriteFailed(err) => write!(f, "Failed to write generated config: {}", err),
        }
    }
}

pub fn gen_and_write(
    clobbering: opts::Clobbering,
    noise_level: opts::NoiseLevel,
    interactivity: opts::Interactivity,
    project_root: impl AsRef<Path>,
    plugins: &PluginMap,
    wrapper: &util::TextWrapper,
) -> Result<(), Error> {
    let shared = {
        let detected = shared::Detected::new().map_err(Error::SharedDetectionFailed)?;
        match interactivity {
            opts::Interactivity::Full => {
                shared::Raw::from_prompt(detected, wrapper).map_err(Error::SharedPromptFailed)
            }
            opts::Interactivity::None => {
                shared::Raw::from_detected(detected).map_err(Error::SharedUpgradeFailed)
            }
        }?
    };
    let project_root = project_root.as_ref();
    let writer = writer::Writer::load_existing(clobbering, project_root, shared)
        .map_err(Error::LoadFailed)?;
    for plugin in plugins.iter() {
        plugin
            .run_and_wait(noise_level, interactivity, &["config-gen"])
            .map_err(|cause| Error::GenFailed {
                plugin_name: plugin.name().to_owned(),
                cause,
            })?;
    }
    writer
        .link_and_write(project_root)
        .map_err(Error::WriteFailed)?;
    Ok(())
}
