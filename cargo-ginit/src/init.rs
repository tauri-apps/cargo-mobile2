use crate::util::take_a_list;
use clap::{App, Arg, ArgMatches, SubCommand};
use ginit::{
    config::Config,
    init::{
        config_gen::interactive_config_gen,
        init,
        steps::{Steps, STEPS},
    },
    opts::Clobbering,
    templating::init_templating,
};

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("init")
        .about("Creates a new project in the current working directory")
        .arg_from_usage("--force 'Clobber files with no remorse'")
        .arg(take_a_list(
            Arg::with_name("only")
                .long("only")
                .help("Only do some steps")
                .value_name("STEPS"),
            STEPS,
        ))
        .arg(take_a_list(
            Arg::with_name("skip")
                .long("skip")
                .help("Skip some steps")
                .value_name("STEPS"),
            STEPS,
        ))
}

#[derive(Debug)]
pub struct InitCommand {
    clobbering: Clobbering,
    only: Option<Steps>,
    skip: Option<Steps>,
}

impl InitCommand {
    pub fn parse<'a>(matches: ArgMatches<'a>) -> Self {
        let clobbering = if matches.is_present("force") {
            Clobbering::Allow
        } else {
            Clobbering::Forbid
        };
        let only = matches
            .args
            .get("only")
            .map(|only| Steps::from(only.vals.as_slice()));
        let skip = matches
            .args
            .get("skip")
            .map(|skip| Steps::from(skip.vals.as_slice()));
        Self {
            clobbering,
            only,
            skip,
        }
    }

    pub fn exec(self, config: Option<&Config>) {
        let new_config = if config.is_none() {
            let old_bike = init_templating(None);
            interactive_config_gen(&old_bike);
            Some(
                Config::load(".")
                    .expect("failed to load config")
                    .expect("no config found - did generation fail?"),
            )
        } else {
            None
        };
        let config = config.unwrap_or_else(|| new_config.as_ref().unwrap());
        let new_bike = init_templating(Some(&config));
        init(&config, &new_bike, self.clobbering, self.only, self.skip);
    }
}
