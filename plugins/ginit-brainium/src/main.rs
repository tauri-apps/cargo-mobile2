#![forbid(unsafe_code)]

mod project;

use ginit_core::{
    config::{self, empty, ConfigTrait as _},
    util::{
        self,
        cli::{self, mixins, NonZeroExit},
    },
};
use structopt::StructOpt;

static NAME: &'static str = "brainium";

#[derive(Debug, StructOpt)]
#[structopt(settings = mixins::SETTINGS)]
pub struct Input {
    #[structopt(flatten)]
    flags: mixins::GlobalFlags,
    #[structopt(subcommand)]
    command: mixins::Barebones,
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
            flags: mixins::GlobalFlags { interactivity, .. },
            command,
        } = self;
        match command {
            mixins::Barebones::ConfigGen => {
                config::gen::detect_or_prompt::<empty::Raw>(interactivity, wrapper, NAME)
                    .map_err(util::display)
            }
            mixins::Barebones::Init {
                clobbering: mixins::Clobbering { clobbering },
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

fn main() {
    NonZeroExit::exec::<Input>(NAME)
}
