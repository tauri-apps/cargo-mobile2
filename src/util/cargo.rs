use crate::env::ExplicitEnv;
use std::path::PathBuf;

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
        Self {
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
        self.manifest_path = manifest_path.map(|manifest_path| {
            dunce::canonicalize(manifest_path).expect("Failed to canonicalize manifest path")
        });
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
            if !manifest_path.exists() {
                log::error!("manifest path {:?} doesn't exist!", manifest_path);
            }
            command.add_arg("--manifest-path").add_arg(manifest_path);
        }
        if let Some(target) = self.target {
            // We used to use `util::host_target_triple` to avoid explicitly
            // specifying the default target triple here, since specifying it
            // results in a different `target` subdir being used... however,
            // for reasons noted in `crate::init::exec`, we now favor explicitly
            // specifying `--target` when possible. Though, due to the
            // solution described in the aforementioned function, omitting the
            // default target here wouldn't actually have any negative effect,
            // but it wouldn't accomplish anything either.
            command.add_args(&["--target", target]);
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
