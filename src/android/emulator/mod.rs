mod avd_list;

pub use avd_list::avd_list;

use super::env::Env;
use crate::bossy;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Emulator {
    name: String,
}

impl Emulator {
    fn new(name: String) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn start(self, env: &Env) -> bossy::Result<bossy::Handle> {
        bossy::Command::impure("emulator/emulator")
            .with_current_dir(env.sdk_root())
            .with_args(&["-avd", &self.name])
            .run()
    }
}
