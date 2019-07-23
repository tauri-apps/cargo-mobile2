use crate::{config::Config, util};
use std::collections::BTreeMap;

pub trait TargetTrait: Sized {
    fn all(config: &Config) -> &BTreeMap<String, Self>;

    fn for_name<'a>(config: &'a Config, name: &str) -> Option<&'a Self> {
        Self::all(config).get(name)
    }

    fn for_arch<'a>(config: &'a Config, arch: &str) -> Option<&'a Self> {
        Self::all(config)
            .values()
            .find(|target| target.arch() == arch)
    }

    fn triple(&self) -> &str;
    fn arch(&self) -> &str;

    fn rustup_add(&self) {
        util::rustup_add(self.triple()).expect("Failed to add target via rustup");
    }
}

#[derive(Default)]
pub struct FallbackBehavior<'a, T: TargetTrait> {
    // we use `dyn` so the type doesn't need to be known when this is `None`
    pub get_target: Option<&'a dyn Fn(&'a Config) -> Option<&'a T>>,
    pub all_targets: bool,
}

impl<'a, T: TargetTrait> FallbackBehavior<'a, T> {
    pub fn get_target(f: &'a dyn Fn(&'a Config) -> Option<&'a T>, then_try_all: bool) -> Self {
        FallbackBehavior {
            all_targets: then_try_all,
            get_target: Some(f),
        }
    }

    pub fn all_targets() -> Self {
        FallbackBehavior {
            all_targets: true,
            get_target: None,
        }
    }
}

pub fn get_targets<'a, Iter, I, T>(
    config: &'a Config,
    targets: Option<Iter>,
    fallback: FallbackBehavior<'a, T>,
) -> Option<Vec<&'a T>>
where
    Iter: ExactSizeIterator<Item = &'a I>,
    I: AsRef<str> + 'a,
    T: TargetTrait,
{
    let targets_empty = targets
        .as_ref()
        .map(|targets| targets.len() == 0)
        .unwrap_or(true);
    if !targets_empty {
        Some(
            targets
                .unwrap()
                .map(|name| T::for_name(config, name.as_ref()).expect("Invalid target"))
                .collect(),
        )
    } else {
        fallback
            .get_target
            .and_then(|get_target| get_target(config))
            .map(|target| vec![target])
            .or_else(|| Some(T::all(config).values().collect()))
    }
}

pub fn call_for_targets<'a, Iter, I, T, F>(
    config: &'a Config,
    targets: Option<Iter>,
    fallback: FallbackBehavior<'a, T>,
    f: F,
) where
    Iter: ExactSizeIterator<Item = &'a I>,
    I: AsRef<str> + 'a,
    T: TargetTrait,
    F: Fn(&T),
{
    let targets = get_targets(config, targets, fallback).expect("No valid targets specified");
    for target in targets {
        f(target);
    }
}
