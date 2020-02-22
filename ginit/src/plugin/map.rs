use super::{LoadError, Plugin};
use ginit_core::config::shared::Shared;

#[derive(Debug, Default)]
pub struct Map {
    plugins: indexmap::IndexMap<String, Plugin>,
}

impl Map {
    pub fn from_iter(plugins: impl Iterator<Item = impl Into<String>>) -> Result<Self, LoadError> {
        plugins
            .map(|name| {
                let name = name.into();
                Plugin::new(&name).map(|plugin| (name, plugin))
            })
            .collect::<Result<_, _>>()
            .map(|plugins| Self { plugins })
    }

    pub fn from_shared(shared: &Shared) -> Result<Self, LoadError> {
        Self::from_iter(shared.plugins().into_iter())
    }

    pub fn get(&self, name: impl AsRef<str>) -> Option<&Plugin> {
        self.plugins.get(name.as_ref())
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Plugin> {
        self.plugins.values()
    }
}
