mod project;

use ginit_core::{
    config::{ConfigTrait as _, EmptyConfig},
    opts,
    protocol::Features,
    templating, PluginTrait,
};

#[derive(Debug, Default)]
pub struct Brainium {
    config: Option<EmptyConfig>,
}

impl PluginTrait for Brainium {
    const NAME: &'static str = "brainium";
    const DESCRIPTION: &'static str = "Brainium-specific shenanigans";
    const FEATURES: Features = Features::BASIC;

    type Config = EmptyConfig;
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
