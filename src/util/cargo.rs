use crate::env::ExplicitEnv;
use once_cell_regex::regex;
use std::path::PathBuf;

fn detect_host() -> Option<String> {
    // TODO: add fast paths
    let result = bossy::Command::impure("rustc")
        .with_args(&["--verbose", "--version"])
        .run_and_wait_for_output();
    match result {
        Ok(output) => match output.stdout_str() {
            Ok(raw) => {
                let re = regex!(r"host: ([\w-]+)");
                let triple = re.captures(raw).map(|caps| caps[0].to_owned());
                if triple.is_none() {
                    log::error!("when detecting host, no matches were found");
                }
                triple
            }
            Err(err) => {
                log::error!("when detecting host, output wasn't valid utf-8: {}", err);
                None
            }
        },
        Err(err) => {
            log::error!("failed to detect host: {}", err);
            None
        }
    }
}

#[derive(Debug)]
pub struct CargoCommand<'a> {
    subcommand: &'a str,
    verbose: bool,
    package: Option<&'a str>,
    manifest_path: Option<PathBuf>,
    target: Option<&'a str>,
    no_default_features: bool,
    features: Option<&'a [String]>,
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
            no_default_features: Default::default(),
            features: Default::default(),
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

    pub fn with_no_default_features(mut self, no_default_features: bool) -> Self {
        self.no_default_features = no_default_features;
        self
    }

    pub fn with_features(mut self, features: Option<&'a [String]>) -> Self {
        self.features = features;
        self
    }

    pub fn with_release(mut self, release: bool) -> Self {
        self.release = release;
        self
    }

    fn into_command_inner(self, mut command: bossy::Command) -> bossy::Command {
        command.add_arg(self.subcommand);
        if self.verbose {
            command.add_arg("-vv");
        }
        if let Some(package) = self.package {
            command.add_args(&["--package", package]);
        }
        if let Some(manifest_path) = self.manifest_path {
            command.add_arg("--manifest-path").add_arg(manifest_path);
        }
        if let Some(target) = self.target {
            // Don't pass target if it's the default target, since that would
            // result in a different build cache being used than with regular
            // `cargo build` stuff.
            if detect_host().as_deref() != self.target {
                command.add_args(&["--target", target]);
            } else {
                log::info!(
                    "omitting explicit target triple {:?}, since it's the default target triple on this host",
                    target
                );
            }
        }
        if self.no_default_features {
            command.add_arg("--no-default-features");
        }
        if let Some(features) = self.features {
            command.add_args(&["--features", &features.join(" ")]);
        }
        if self.release {
            command.add_arg("--release");
        }
        command
    }

    pub fn into_command_impure(self) -> bossy::Command {
        self.into_command_inner(bossy::Command::impure("cargo"))
    }

    pub fn into_command_pure(self, env: &impl ExplicitEnv) -> bossy::Command {
        self.into_command_inner(bossy::Command::pure("cargo").with_env_vars(env.explicit_env()))
    }
}
