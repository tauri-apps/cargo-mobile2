mod config;
mod project;

use ginit_core::{config::ConfigTrait as _, ipc, opts, templating, PluginTrait};

#[derive(Debug, Default)]
pub struct Brainium {
    config: Option<config::Config>,
}

impl PluginTrait for Brainium {
    const NAME: &'static str = "brainium";
    const DESCRIPTION: &'static str = "Brainium-specific shenanigans";

    type Config = config::Config;
    fn update_config(&mut self, config: Self::Config) {
        self.config = Some(config);
    }

    type InitError = project::Error;
    fn init(&mut self, clobbering: opts::Clobbering) -> Result<(), Self::InitError> {
        let config = self.config();
        let bike = templating::init(Some(config.shared()));
        project::generate(config, &bike, clobbering)
    }

    type ExecError = String;
}

impl Brainium {
    fn config(&self) -> &<Self as PluginTrait>::Config {
        self.config.as_ref().unwrap()
    }
}

fn main() {
    ipc::listen(&mut Brainium::default()).expect("uh-oh");
}
