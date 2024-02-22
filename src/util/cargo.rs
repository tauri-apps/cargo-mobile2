use std::{collections::HashMap, ffi::OsString, path::PathBuf};

use crate::{env::ExplicitEnv, DuctExpressionExt};

#[derive(Debug)]
pub struct CargoCommand<'a> {
    subcommand: &'a str,
    verbose: bool,
    package: Option<&'a str>,
    manifest_path: Option<PathBuf>,
    target: Option<&'a str>,
    no_default_features: bool,
    features: Option<&'a [String]>,
    args: Option<&'a [String]>,
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
            args: Default::default(),
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

    pub fn with_args(mut self, args: Option<&'a [String]>) -> Self {
        self.args = args;
        self
    }

    pub fn with_release(mut self, release: bool) -> Self {
        self.release = release;
        self
    }

    pub fn build(self, env: &impl ExplicitEnv) -> duct::Expression {
        let mut args = vec![self.subcommand.to_owned()];
        if self.verbose {
            args.push("-vv".into());
        }
        if let Some(package) = self.package {
            args.extend_from_slice(&["--package".into(), package.to_owned()]);
        }
        if let Some(manifest_path) = self.manifest_path {
            if !manifest_path.exists() {
                log::error!("manifest path {:?} doesn't exist!", manifest_path);
            }
            args.extend_from_slice(&[
                "--manifest-path".into(),
                manifest_path.to_string_lossy().to_string(),
            ]);
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
            args.extend_from_slice(&["--target".into(), target.to_owned()]);
        }
        if self.no_default_features {
            args.push("--no-default-features".into());
        }
        if let Some(features) = self.features {
            let features = features.join(" ");
            args.extend_from_slice(&["--features".into(), features.as_str().to_string()]);
        }
        if let Some(a) = self.args {
            args.extend_from_slice(a);
        }
        if self.release {
            args.push("--release".into());
        }

        duct::cmd("cargo", args)
            .vars(env.explicit_env())
            .vars(explicit_cargo_env())
            .dup_stdio()
    }
}

fn explicit_cargo_env() -> HashMap<String, OsString> {
    let mut vars = HashMap::new();
    if let Some(target_dir) = std::env::var_os("CARGO_TARGET_DIR") {
        vars.insert("CARGO_TARGET_DIR".into(), target_dir);
    }
    if let Some(target_dir) = std::env::var_os("CARGO_BUILD_TARGET_DIR") {
        vars.insert("CARGO_BUILD_TARGET_DIR".into(), target_dir);
    }
    vars
}
