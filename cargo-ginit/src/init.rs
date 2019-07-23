use crate::util::take_a_list;
use clap::{App, Arg, ArgMatches, SubCommand};
use ginit::{
    config::Config,
    init::{config::interactive_config_gen, init, Skip, STEPS},
    templating::init_templating,
};

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("init")
        .about("Creates a new project in the current working directory")
        .arg_from_usage("--force 'Clobber files with no remorse'")
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
    force: bool,
    skip: Skip,
}

impl InitCommand {
    pub fn parse<'a>(matches: ArgMatches<'a>) -> Self {
        let force = matches.is_present("force");
        let skip = matches
            .args
            .get("skip")
            .map(|skip| Skip::from(skip.vals.as_slice()))
            .unwrap_or_default();
        Self { force, skip }
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
        init(&config, &new_bike, self.force, self.skip);
    }
}
