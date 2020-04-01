use crate::util;
use once_cell_regex::exports::once_cell::sync::OnceCell;
use std::{
    collections::BTreeMap,
    fmt::{self, Debug, Display},
};

pub trait TargetTrait<'a>: Debug + Sized {
    const DEFAULT_KEY: &'static str;

    fn all() -> &'a BTreeMap<&'a str, Self>;

    fn name_list() -> &'static [&'a str]
    where
        Self: 'static,
    {
        static INSTANCE: OnceCell<Vec<&str>> = OnceCell::new();
        INSTANCE.get_or_init(|| Self::all().keys().map(|key| *key).collect::<Vec<_>>())
    }

    fn default_ref() -> &'a Self {
        Self::all()
            .get(Self::DEFAULT_KEY)
            .expect("developer error: no target matched `DEFAULT_KEY`")
    }

    fn for_name(name: &str) -> Option<&'a Self> {
        Self::all().get(name)
    }

    fn for_arch(arch: &str) -> Option<&'a Self> {
        Self::all().values().find(|target| target.arch() == arch)
    }

    fn triple(&'a self) -> &'a str;

    fn arch(&'a self) -> &'a str;

    fn install(&'a self) -> bossy::Result<bossy::ExitStatus> {
        util::rustup_add(self.triple())
    }

    fn install_all() -> bossy::Result<()>
    where
        Self: 'a,
    {
        for target in Self::all().values() {
            target.install()?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct TargetInvalid {
    name: String,
    possible: Vec<String>,
}

impl Display for TargetInvalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Target {:?} is invalid; the possible targets are {:?}",
            self.name, self.possible,
        )
    }
}

pub fn get_targets<'a, Iter, I, T, U>(
    targets: Iter,
    // we use `dyn` so the type doesn't need to be known when this is `None`
    fallback: Option<(&'a dyn Fn(U) -> Option<&'a T>, U)>,
) -> Result<Vec<&'a T>, TargetInvalid>
where
    Iter: ExactSizeIterator<Item = &'a I>,
    I: AsRef<str> + 'a,
    T: TargetTrait<'a>,
{
    let targets_empty = targets.len() == 0;
    Ok(if !targets_empty {
        targets
            .map(|name| {
                T::for_name(name.as_ref()).ok_or_else(|| TargetInvalid {
                    name: name.as_ref().to_owned(),
                    possible: T::all().keys().map(|key| key.to_string()).collect(),
                })
            })
            .collect::<Result<_, _>>()?
    } else {
        let target = fallback
            .and_then(|(get_target, arg)| get_target(arg))
            .unwrap_or_else(|| {
                log::info!("falling back on default target ({})", T::DEFAULT_KEY);
                T::default_ref()
            });
        vec![target]
    })
}

pub fn call_for_targets_with_fallback<'a, Iter, I, T, U, E, F>(
    targets: Iter,
    fallback: &'a dyn Fn(U) -> Option<&'a T>,
    arg: U,
    f: F,
) -> Result<Result<(), E>, TargetInvalid>
where
    Iter: ExactSizeIterator<Item = &'a I>,
    I: AsRef<str> + 'a,
    T: TargetTrait<'a>,
    F: Fn(&T) -> Result<(), E>,
{
    get_targets(targets, Some((fallback, arg))).map(|targets| {
        for target in targets {
            f(target)?;
        }
        Ok(())
    })
}

pub fn call_for_targets<'a, Iter, I, T, E, F>(
    targets: Iter,
    f: F,
) -> Result<Result<(), E>, TargetInvalid>
where
    Iter: ExactSizeIterator<Item = &'a I>,
    I: AsRef<str> + 'a,
    T: TargetTrait<'a> + 'a,
    F: Fn(&T) -> Result<(), E>,
{
    get_targets::<_, _, _, ()>(targets, None).map(|targets| {
        for target in targets {
            f(target)?;
        }
        Ok(())
    })
}
