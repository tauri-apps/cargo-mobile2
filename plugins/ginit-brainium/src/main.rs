#![forbid(unsafe_code)]

mod project;

use ginit_core::{
    config::{self, empty, ConfigTrait as _},
    util::{self, cli},
};
use structopt::StructOpt;

static NAME: &'static str = "brainium";

#[cli::main(NAME)]
#[derive(Debug, StructOpt)]
#[structopt(settings = cli::SETTINGS)]
pub struct Input {
    #[structopt(flatten)]
    flags: cli::GlobalFlags,
    #[structopt(subcommand)]
    command: cli::Barebones,
}

impl cli::Exec for Input {
    type Config = empty::Config;
    type Error = String;

    fn exec(
        self,
        config: Option<Self::Config>,
        wrapper: &util::TextWrapper,
    ) -> Result<(), Self::Error> {
        let Self {
            flags: cli::GlobalFlags { interactivity, .. },
            command,
        } = self;
        match command {
            cli::Barebones::ConfigGen => {
                config::gen::detect_or_prompt::<empty::Raw>(interactivity, wrapper, NAME)
                    .map_err(util::display)
            }
            cli::Barebones::Init {
                clobbering: cli::Clobbering { clobbering },
            } => {
                let config = config.as_ref().ok_or_else(|| {
                    "Plugin is unconfigured, but configuration is required for this command."
                        .to_owned()
                })?;
                project::gen(config, &config.init_templating(), clobbering).map_err(util::display)
            }
        }
    }
}
