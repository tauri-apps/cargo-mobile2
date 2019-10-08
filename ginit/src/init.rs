use crate::{
    config::Umbrella,
    core::{exports::into_result::command::CommandError, opts, util},
    plugin::{Configured, Error as PluginError, Plugin},
    steps::{Registry as StepRegistry, StepNotRegistered, Steps},
};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    // CargoConfigGenFailed(cargo::GenError),
    // CargoConfigWriteFailed(cargo::WriteError),
    // HelloWorldGenFailed(rust::Error),
    // AndroidGenFailed(android::project::Error),
    // IosDepsFailed(IosDepsError),
    // IosGenFailed(ios::project::Error),
    OnlyParseFailed(StepNotRegistered),
    SkipParseFailed(StepNotRegistered),
    StepNotRegistered(StepNotRegistered),
    PluginFailed {
        plugin_name: String,
        cause: PluginError,
    },
    OpenInEditorFailed(CommandError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Error::CargoConfigGenFailed(err) => {
            //     write!(f, "Failed to generate \".cargo/config\": {}", err)
            // }
            // Error::CargoConfigWriteFailed(err) => {
            //     write!(f, "Failed to write \".cargo/config\": {}", err)
            // }
            // Error::HelloWorldGenFailed(err) => {
            //     write!(f, "Failed to generate hello world project: {}", err)
            // }
            // Error::AndroidGenFailed(err) => {
            //     write!(f, "Failed to generate Android project: {}", err)
            // }
            // Error::IosDepsFailed(err) => write!(f, "Failed to install iOS dependencies: {}", err),
            // Error::IosGenFailed(err) => write!(f, "Failed to generate iOS project: {}", err),
            Error::OnlyParseFailed(err) => write!(f, "Failed to parse `only` step list: {}", err),
            Error::SkipParseFailed(err) => write!(f, "Failed to parse `skip` step list: {}", err),
            Error::StepNotRegistered(err) => write!(f, "{}", err),
            Error::PluginFailed{
                plugin_name,
                cause,
            } => write!(f, "Failed to init {:?} plugin: {}", plugin_name, cause),
            Error::OpenInEditorFailed(err) => write!(f, "Failed to open project in editor (your project generated successfully though, so no worries!): {}", err),
        }
    }
}

pub fn init<'a>(
    plugins: impl Iterator<Item = &'a Plugin<Configured>> + Clone,
    clobbering: opts::Clobbering,
    open_in: opts::OpenIn,
    only: Option<&[impl AsRef<str>]>,
    skip: Option<&[impl AsRef<str>]>,
) -> Result<(), Error> {
    let step_registry = {
        let mut registry = StepRegistry::default();
        for plugin in plugins.clone() {
            registry.register(plugin.name());
        }
        registry
    };
    let steps = {
        let only = only
            .map(|only| Steps::parse(&step_registry, only))
            .unwrap_or_else(|| Ok(Steps::new_all_set(&step_registry)))
            .map_err(Error::OnlyParseFailed)?;
        let skip = skip
            .map(|skip| Steps::parse(&step_registry, skip))
            .unwrap_or_else(|| Ok(Steps::new_all_unset(&step_registry)))
            .map_err(Error::SkipParseFailed)?;
        Steps::from_bits(&step_registry, only.bits() & !skip.bits())
    };
    // if steps.is_set("cargo") {
    //     CargoConfig::generate(config, &steps)
    //         .map_err(Error::CargoConfigGenFailed)?
    //         .write(&config)
    //         .map_err(Error::CargoConfigWriteFailed)?;
    // }
    for plugin in plugins {
        if steps
            .is_set(plugin.name())
            .map_err(Error::StepNotRegistered)?
        {
            plugin
                .init(clobbering)
                .map_err(|cause| Error::PluginFailed {
                    plugin_name: plugin.name().to_owned(),
                    cause,
                })?;
        }
    }
    if let opts::OpenIn::Editor = open_in {
        util::open_in_editor(".").map_err(Error::OpenInEditorFailed)?;
    }
    Ok(())
}
