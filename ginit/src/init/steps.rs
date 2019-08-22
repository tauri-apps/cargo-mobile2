use crate::util::FriendlyContains;

pub static STEPS: &'static [&'static str] = &[
    "deps",
    "toolchains",
    "cargo",
    "hello_world",
    "android",
    "ios",
];

#[derive(Clone, Copy, Debug, Default)]
pub struct Steps {
    pub deps: bool,
    pub toolchains: bool,
    pub cargo: bool,
    pub hello_world: bool,
    pub android: bool,
    pub ios: bool,
}

impl<'a, T> From<&'a [T]> for Steps
where
    &'a [T]: FriendlyContains<T>,
    str: PartialEq<T>,
{
    fn from(steps: &'a [T]) -> Self {
        Self {
            deps: steps.friendly_contains("deps"),
            toolchains: steps.friendly_contains("toolchains"),
            cargo: steps.friendly_contains("cargo"),
            hello_world: steps.friendly_contains("hello_world"),
            android: steps.friendly_contains("android"),
            ios: steps.friendly_contains("ios"),
        }
    }
}

impl Steps {
    fn map_steps(mut self, f: impl Fn(bool) -> bool) -> Self {
        for step in &mut [
            &mut self.deps,
            &mut self.toolchains,
            &mut self.cargo,
            &mut self.hello_world,
            &mut self.android,
            &mut self.ios,
        ] {
            **step = f(**step);
        }
        self
    }

    fn zip_map_steps(mut self, other: impl Into<Self>, f: impl Fn(bool, bool) -> bool) -> Self {
        let other = other.into();
        for (step, other_step) in &mut [
            (&mut self.deps, other.deps),
            (&mut self.toolchains, other.toolchains),
            (&mut self.cargo, other.cargo),
            (&mut self.hello_world, other.hello_world),
            (&mut self.android, other.android),
            (&mut self.ios, other.ios),
        ] {
            **step = f(**step, *other_step);
        }
        self
    }

    pub fn all(state: bool) -> Self {
        Self::default().map_steps(|_| state)
    }

    pub fn not(self) -> Self {
        self.map_steps(|state| !state)
    }

    pub fn and(self, other: impl Into<Self>) -> Self {
        self.zip_map_steps(other, |a, b| a && b)
    }
}
