use crate::util::pure_command::{ExplicitEnv, PureCommand};
use std::{path::PathBuf, process::Command};

#[derive(Debug)]
pub struct CargoCommand<'a> {
    subcommand: &'a str,
    verbose: bool,
    package: Option<&'a str>,
    manifest_path: Option<PathBuf>,
    target: Option<&'a str>,
    features: Option<&'a str>,
    no_default_features: bool,
    release: bool,
}

impl<'a> CargoCommand<'a> {
    pub fn new(subcommand: &'a str) -> Self {
        CargoCommand {
            subcommand,
            verbose: Default::default(),
            package: Default::default(),
            manifest_path: Default::default(),
            target: Default::default(),
            features: Default::default(),
            no_default_features: Default::default(),
            release: Default::default(),
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn with_package(mut self, package: Option<&'a str>) -> Self {
        self.package = package;
        self
    }

    pub fn with_manifest_path(mut self, manifest_path: Option<PathBuf>) -> Self {
        self.manifest_path = manifest_path;
        self
    }

    pub fn with_target(mut self, target: Option<&'a str>) -> Self {
        self.target = target;
        self
    }

    pub fn with_features(mut self, features: Option<&'a str>) -> Self {
        self.features = features;
        self
    }

    pub fn with_no_default_features(mut self, no_default_features: bool) -> Self {
        self.no_default_features = no_default_features;
        self
    }

    pub fn with_release(mut self, release: bool) -> Self {
        self.release = release;
        self
    }

    fn into_command_inner(self, mut command: Command) -> Command {
        command.arg(self.subcommand);
        if self.verbose {
            command.arg("-vv");
        }
        if let Some(package) = self.package {
            command.args(&["--package", package]);
        }
        if let Some(manifest_path) = self.manifest_path {
            command.arg("--manifest-path").arg(manifest_path);
        }
        if let Some(target) = self.target {
            command.args(&["--target", target]);
        }
        if let Some(features) = self.features {
            command.args(&["--features", features]);
        }
        if self.no_default_features {
            command.arg("--no-default-features");
        }
        if self.release {
            command.arg("--release");
        }
        command
    }

    pub fn into_command_impure(self) -> Command {
        self.into_command_inner(Command::new("cargo"))
    }

    pub fn into_command(self, env: &impl ExplicitEnv) -> Command {
        self.into_command_inner(PureCommand::new("cargo", env))
    }
}
