#![forbid(unsafe_code)]

mod project;

use ginit_core::{
    cli_app,
    config::{self, empty, umbrella::Umbrella, ConfigTrait as _},
    util::cli::{self, Barebones, NonZeroExit},
};

static NAME: &'static str = "brainium";

fn main() {
    NonZeroExit::main(|wrapper| {
        let app = cli_app!();
        let input =
            cli::get_matches_and_parse::<Barebones>(app, NAME).map_err(NonZeroExit::Clap)?;
        let config = Umbrella::load_plugin(NAME).map_err(NonZeroExit::display)?;
        match input.command {
            Barebones::ConfigGen => {
                config::gen::detect_or_prompt::<empty::Raw>(input.interactivity, wrapper, NAME)
                    .map_err(NonZeroExit::display)
            }
            Barebones::Init { clobbering } => {
                let config = config.as_ref().ok_or_else(|| {
                    NonZeroExit::display(
                        "Plugin is unconfigured, but configuration is required for this command.",
                    )
                })?;
                project::gen(config, &config.init_templating(), clobbering)
                    .map_err(NonZeroExit::display)
            }
        }
    })
}
