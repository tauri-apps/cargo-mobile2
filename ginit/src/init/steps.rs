use crate::util::FriendlyContains;

pub static STEPS: &'static [&'static str] = &[
    "deps",
    "toolchains",
    "cargo",
    "hello-world",
    "android",
    "ios",
];

bitflags::bitflags! {
    pub struct Steps: u32 {
        const DEPS = 0b00000001;
        const TOOLCHAINS = 0b00000010;
        const CARGO = 0b00000100;
        const HELLO_WORLD = 0b00001000;
        const ANDROID = 0b00010000;
        const IOS = 0b00100000;
    }
}

impl<'a, T> From<&'a [T]> for Steps
where
    &'a [T]: FriendlyContains<T>,
    str: PartialEq<T>,
{
    fn from(steps: &'a [T]) -> Self {
        let mut flags = Self::empty();
        if steps.friendly_contains("deps") {
            flags |= Self::DEPS;
        }
        if steps.friendly_contains("toolchains") {
            flags |= Self::TOOLCHAINS;
        }
        if steps.friendly_contains("cargo") {
            flags |= Self::CARGO;
        }
        if steps.friendly_contains("hello-world") {
            flags |= Self::HELLO_WORLD;
        }
        if steps.friendly_contains("android") {
            flags |= Self::ANDROID;
        }
        if steps.friendly_contains("ios") {
            flags |= Self::IOS;
        }
        flags
    }
}
