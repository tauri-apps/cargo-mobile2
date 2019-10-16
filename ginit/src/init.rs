use crate::{
    core::{exports::into_result::command::CommandError, opts, util},
    plugin::{Map as PluginMap, RunError as PluginError},
    steps::{Registry as StepRegistry, StepNotRegistered, Steps},
};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
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
    plugins: &PluginMap,
    noise_level: opts::NoiseLevel,
    interactivity: opts::Interactivity,
    clobbering: opts::Clobbering,
    open_in: opts::OpenIn,
    only: Option<&[impl AsRef<str>]>,
    skip: Option<&[impl AsRef<str>]>,
) -> Result<(), Error> {
    let step_registry = {
        let mut registry = StepRegistry::default();
        for plugin in plugins.iter() {
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
    let args = {
        let mut args = vec!["init"];
        if let opts::Clobbering::Allow = clobbering {
            args.push("--force");
        }
        args
    };
    for plugin in plugins.iter().filter(|plugin| plugin.supports("init")) {
        if steps
            .is_set(plugin.name())
            .map_err(Error::StepNotRegistered)?
        {
            plugin
                .run(noise_level, interactivity, &args)
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
