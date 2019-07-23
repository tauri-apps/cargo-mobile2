use std::{path::PathBuf, process::Command};

#[derive(Debug)]
pub struct CargoCommand<'a> {
    subcommand: &'a str,
    verbose: bool,
    package: Option<&'a str>,
    manifest_path: Option<PathBuf>,
    target: Option<&'a str>,
    features: Option<&'a str>,
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
            release: Default::default(),
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> CargoCommand<'a> {
        self.verbose = verbose;
        self
    }

    pub fn with_package(mut self, package: Option<&'a str>) -> CargoCommand {
        self.package = package;
        self
    }

    pub fn with_manifest_path(mut self, manifest_path: Option<PathBuf>) -> Self {
        self.manifest_path = manifest_path;
        self
    }

    pub fn with_target(mut self, target: Option<&'a str>) -> CargoCommand {
        self.target = target;
        self
    }

    pub fn with_features(mut self, features: Option<&'a str>) -> CargoCommand {
        self.features = features;
        self
    }

    pub fn with_release(mut self, release: bool) -> CargoCommand<'a> {
        self.release = release;
        self
    }

    pub fn into_command(self) -> Command {
        let mut command = Command::new("cargo");
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
        if self.release {
            command.arg("--release");
        }
        command
    }
}
