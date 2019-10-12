use super::{NewError, Plugin};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Map {
    plugins: HashMap<String, Plugin>,
}

impl Map {
    pub fn add(&mut self, name: impl AsRef<str>) -> Result<(), NewError> {
        let name = name.as_ref();
        Plugin::new(name).map(|plugin| {
            self.plugins.insert(name.to_owned(), plugin);
        })
    }

    pub fn get(&self, name: impl AsRef<str>) -> Option<&Plugin> {
        self.plugins.get(name.as_ref())
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Plugin> {
        self.plugins.values()
    }
}
