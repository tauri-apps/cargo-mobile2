pub mod lfs;
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

    pub fn command(&self) -> duct::Expression {
        duct::cmd(
            "git",
            ["-C", self.root.as_os_str().to_str().unwrap_or_default()],
        )
    }

    pub fn command_parse(&self, arg_str: impl AsRef<str>) -> duct::Expression {
        let mut args = vec!["-C", self.root.as_os_str().to_str().unwrap_or_default()];
        for arg in arg_str.as_ref().split(' ') {
            args.push(arg)
        }
        duct::cmd("git", args)
    }

    pub fn init(&self) -> std::io::Result<()> {
        if !self.root.join(".git").exists() {
            self.command()
                .before_spawn(|cmd| {
                    cmd.arg("init");
                    Ok(())
                })
                .run()?;
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

    pub fn user_name(&self) -> std::io::Result<String> {
        self.command()
            .before_spawn(|cmd| {
                cmd.args(["config", "user.name"]);
                Ok(())
            })
            .read()
    }

    pub fn user_email(&self) -> std::io::Result<String> {
        self.command()
            .before_spawn(|cmd| {
                cmd.args(["config", "user.email"]);
                Ok(())
            })
            .read()
    }
}
