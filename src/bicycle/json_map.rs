use handlebars::to_json;
use serde::Serialize;
use serde_json::value::{Map, Value as Json};

/// Map of template variable names and values.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct JsonMap(pub(crate) Map<String, Json>);

impl Default for JsonMap {
    fn default() -> Self {
        Self(Map::new())
    }
}

impl JsonMap {
    pub fn insert(&mut self, name: &str, value: impl Serialize) {
        self.0.insert(name.to_owned(), to_json(value));
    }
}
