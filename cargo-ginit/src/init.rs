use crate::{plugins::PluginMap, util::take_a_list};
use clap::{App, Arg, ArgMatches, SubCommand};
use ginit::{
    config::Umbrella,
    core::opts::{Clobbering, OpenIn},
    init,
    plugin::Configured,
    steps::{Registry as StepRegistry, StepNotRegistered, Steps},
};

pub fn subcommand<'a, 'b>(steps: &'a [&'a str]) -> App<'a, 'b> {
    SubCommand::with_name("init")
        .about("Creates a new project in the current working directory")
        .arg_from_usage("--force 'Clobber files with no remorse'")
        .arg_from_usage("--open 'Open in VS Code'")
        .arg(take_a_list(
            Arg::with_name("only")
                .long("only")
                .help("Only do some steps")
                .value_name("STEPS"),
            steps,
        ))
        .arg(take_a_list(
            Arg::with_name("skip")
                .long("skip")
                .help("Skip some steps")
                .value_name("STEPS"),
            steps,
        ))
}

#[derive(Debug)]
pub struct InitCommand {
    clobbering: Clobbering,
    open_in: OpenIn,
    only: Option<Vec<String>>,
    skip: Option<Vec<String>>,
}

impl InitCommand {
    pub fn parse<'a>(matches: ArgMatches<'a>) -> Self {
        let clobbering = if matches.is_present("force") {
            Clobbering::Allow
        } else {
            Clobbering::Forbid
        };
        let open_in = if matches.is_present("open") {
            OpenIn::Editor
        } else {
            OpenIn::Nothing
        };
        let only = matches.args.get("only").map(|only| {
            only.vals
                .iter()
                .map(|step| step.to_string_lossy().into_owned())
                .collect()
        });
        let skip = matches.args.get("skip").map(|skip| {
            skip.vals
                .iter()
                .map(|step| step.to_string_lossy().into_owned())
                .collect()
        });
        Self {
            clobbering,
            open_in,
            only,
            skip,
        }
    }

    pub fn exec(
        self,
        config: &Umbrella,
        plugins: &PluginMap<Configured>,
    ) -> Result<(), init::Error> {
        init::init(
            plugins.client(),
            plugins.iter(),
            self.clobbering,
            self.open_in,
            self.only.as_ref().map(|only| only.as_slice()),
            self.skip.as_ref().map(|skip| skip.as_slice()),
        )
    }
}
