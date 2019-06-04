use crate::{Config, util};
use std::collections::BTreeMap;

pub trait TargetTrait: Sized {
    fn all() -> &'static BTreeMap<String, Self>;

    fn for_name(name: &str) -> Option<&'static Self> {
        Self::all().get(name)
    }

    fn for_arch(arch: &str) -> Option<&'static Self> {
        Self::all().values().find(|target| target.arch() == arch)
    }

    fn triple(&self) -> &str;
    fn arch(&self) -> &str;

    fn rustup_add(&self) {
        util::rustup_add(self.triple()).expect("Failed to add target via rustup");
    }
}

#[derive(Default)]
pub struct FallbackBehavior<T>
where
    T: 'static + TargetTrait,
{
    // we use `dyn` so the type doesn't need to be known when this is `None`
    pub get_target: Option<&'static dyn Fn() -> Option<&'static T>>,
    pub all_targets: bool,
}

impl<T> FallbackBehavior<T>
where
    T: 'static + TargetTrait,
{
    pub fn get_target(
        f: &'static dyn Fn() -> Option<&'static T>,
        then_try_all: bool,
    ) -> Self {
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

pub fn get_targets<T>(
    targets: Option<Vec<String>>,
    fallback: FallbackBehavior<T>,
) -> Option<Vec<&'static T>>
where
    T: 'static + TargetTrait,
{
    let targets_empty = targets
        .as_ref()
        .map(|targets| targets.is_empty())
        .unwrap_or(true);
    if !targets_empty {
        Some(targets
            .unwrap()
            .iter()
            .map(String::as_str)
            .map(|name| T::for_name(name).expect("Invalid target"))
            .collect())
    } else {
        fallback
            .get_target
            .and_then(|get_target| get_target())
            .map(|target| vec![target])
            .or_else(|| Some(T::all().values().collect()))
    }
}

pub fn get_possible_values<T: 'static + TargetTrait>() -> Vec<&'static str> {
    if Config::exists() {
        T::all()
            .keys()
            .map(String::as_str)
            .collect()
    } else {
        vec![]
    }
}

pub fn call_for_targets<T, F>(
    targets: Option<Vec<String>>,
    fallback: FallbackBehavior<T>,
    f: F,
)
where
    T: 'static + TargetTrait,
    F: Fn(&T),
{
    let targets = get_targets(targets, fallback)
        .expect("No valid targets specified");
    for target in targets {
        f(target);
    }
}
