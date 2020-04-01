pub mod submodule;

use std::{fs, io, path::Path};

#[derive(Clone, Copy, Debug)]
pub struct Git<'a> {
    root: &'a Path,
}

impl<'a> Git<'a> {
    pub fn new(root: &'a Path) -> Self {
        Self { root }
    }

    pub fn command(&self) -> bossy::Command {
        bossy::Command::impure("git")
            .with_arg("-C")
            .with_arg(self.root)
    }

    pub fn init(&self) -> bossy::Result<()> {
        if !self.root.join(".git").exists() {
            self.command().with_arg("init").run_and_wait()?;
        }
        Ok(())
    }

    pub fn config(&self) -> io::Result<Option<String>> {
        let path = self.root.join(".git/config");
        if path.exists() {
            fs::read_to_string(&path).map(Some)
        } else {
            Ok(None)
        }
    }
}
