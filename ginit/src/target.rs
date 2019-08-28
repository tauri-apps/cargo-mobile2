use crate::util;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    pub fn is_debug(self) -> bool {
        self == Profile::Debug
    }

    pub fn is_release(self) -> bool {
        self == Profile::Release
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }
}

pub trait TargetTrait<'a>: Sized {
    const DEFAULT_KEY: &'static str;

    fn all() -> &'a BTreeMap<&'a str, Self>;

    fn default_ref() -> &'a Self {
        Self::all()
            .get(Self::DEFAULT_KEY)
            .expect("No target matched `DEFAULT_KEY`")
    }

    fn for_name(name: &str) -> Option<&'a Self> {
        Self::all().get(name)
    }

    fn for_arch(arch: &str) -> Option<&'a Self> {
        Self::all().values().find(|target| target.arch() == arch)
    }

    fn triple(&'a self) -> &'a str;
    fn arch(&'a self) -> &'a str;

    fn rustup_add(&'a self) {
        util::rustup_add(self.triple()).expect("Failed to add target via rustup");
    }
}

pub fn get_targets<'a, Iter, I, T, U>(
    targets: Iter,
    // we use `dyn` so the type doesn't need to be known when this is `None`
    fallback: Option<(&'a dyn Fn(U) -> Option<&'a T>, U)>,
) -> Option<Vec<&'a T>>
where
    Iter: ExactSizeIterator<Item = &'a I>,
    I: AsRef<str> + 'a,
    T: TargetTrait<'a>,
{
    let targets_empty = targets.len() == 0;
    if !targets_empty {
        Some(
            targets
                .map(|name| T::for_name(name.as_ref()).expect("Invalid target"))
                .collect(),
        )
    } else {
        fallback
            .and_then(|(get_target, arg)| get_target(arg))
            .or_else(|| {
                log::info!("falling back on default target ({})", T::DEFAULT_KEY);
                Some(T::default_ref())
            })
            .map(|target| vec![target])
    }
}

pub fn call_for_targets_with_fallback<'a, Iter, I, T, U, F>(
    targets: Iter,
    fallback: &'a dyn Fn(U) -> Option<&'a T>,
    arg: U,
    f: F,
) where
    Iter: ExactSizeIterator<Item = &'a I>,
    I: AsRef<str> + 'a,
    T: TargetTrait<'a>,
    F: Fn(&T),
{
    let targets = get_targets(targets, Some((fallback, arg))).expect("No valid targets specified");
    for target in targets {
        f(target);
    }
}

pub fn call_for_targets<'a, Iter, I, T, F>(targets: Iter, f: F)
where
    Iter: ExactSizeIterator<Item = &'a I>,
    I: AsRef<str> + 'a,
    T: TargetTrait<'a> + 'a,
    F: Fn(&T),
{
    let targets = get_targets::<_, _, _, ()>(targets, None).expect("No valid targets specified");
    for target in targets {
        f(target);
    }
}
