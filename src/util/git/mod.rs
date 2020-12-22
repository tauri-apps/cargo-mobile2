pub mod repo;
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

    pub fn root(&'a self) -> &'a Path {
        self.root
    }

    pub fn command(&self) -> bossy::Command {
        bossy::Command::impure("git")
            .with_arg("-C")
            .with_arg(self.root)
    }

    pub fn command_parse(&self, arg_str: impl AsRef<str>) -> bossy::Command {
        self.command().with_parsed_args(arg_str)
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

    pub fn modules(&self) -> io::Result<Option<String>> {
        let path = self.root.join(".gitmodules");
        if path.exists() {
            fs::read_to_string(&path).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn user_name(&self) -> bossy::Result<String> {
        self.command()
            .with_args(&["config", "user.name"])
            .run_and_wait_for_string()
    }

    pub fn user_email(&self) -> bossy::Result<String> {
        self.command()
            .with_args(&["config", "user.email"])
            .run_and_wait_for_string()
    }
}
