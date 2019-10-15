mod project;

use ginit_core::{
    cli_app,
    config::{umbrella::Umbrella, ConfigTrait as _},
    util::{
        cli::{self, InitOnly, NonZeroExit},
        TextWrapper,
    },
};

static NAME: &'static str = "brainium";

fn inner(_wrapper: &TextWrapper) -> Result<(), NonZeroExit> {
    let app = cli_app!(NAME);
    let input = cli::get_matches_and_parse::<InitOnly>(app, NAME).map_err(NonZeroExit::Clap)?;
    let config = Umbrella::load_plugin(".", NAME).map_err(NonZeroExit::display)?;
    project::generate(&config, &config.init_templating(), input.command.clobbering)
        .map_err(NonZeroExit::display)
}

fn main() {
    NonZeroExit::main(inner)
}
