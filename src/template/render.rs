use crate::CONFIG;
use derive_more::From;
use handlebars::{Handlebars, handlebars_helper, to_json};
use serde::Serialize;
use serde_json::value::{Map, Value as Json};
use std::{ops::Deref, path::Path};

#[derive(Debug, From)]
pub enum RenderError {
    TemplateRenderError(handlebars::TemplateRenderError),
    TemplateFileError(handlebars::TemplateFileError),
    RenderError(handlebars::RenderError),
}

#[derive(Debug)]
pub struct JsonMap(Map<String, Json>);

impl JsonMap {
    pub fn insert(&mut self, key: &str, value: impl Serialize) {
        self.0.insert(key.to_owned(), to_json(value));
    }
}

impl Default for JsonMap {
    fn default() -> Self {
        let mut map = JsonMap(Map::new());
        map.insert("tool_name", &*crate::NAME);
        map
    }
}

impl Deref for JsonMap {
    type Target = Map<String, Json>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn escape(raw: &str) -> String {
    raw.to_owned()
}

handlebars_helper!(prefix_path: |path: str|
    CONFIG.prefix_path(path)
        .to_str()
        .expect("Prefixed path contained invalid unicode")
        .to_owned()
);

handlebars_helper!(unprefix_path: |path: str|
    CONFIG.unprefix_path(path)
        .to_str()
        .expect("Unprefixed path contained invalid unicode")
        .to_owned()
);

fn config_handlebars() -> Handlebars {
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars.register_escape_fn(escape);
    handlebars.register_helper("prefix_path", Box::new(prefix_path));
    handlebars.register_helper("unprefix_path", Box::new(unprefix_path));
    handlebars
}

pub fn render_str(
    template: &str,
    insert_data: impl FnOnce(&mut JsonMap),
) -> Result<String, RenderError> {
    let handlebars = config_handlebars();
    let mut data = JsonMap::default();
    insert_data(&mut data);
    handlebars.render_template(template, &*data).map_err(Into::into)
}

pub fn render_file(
    name: &str,
    src: &Path,
    insert_data: impl FnOnce(&mut JsonMap),
) -> Result<String, RenderError> {
    let mut handlebars = config_handlebars();
    handlebars.register_template_file(name, src)?;
    let mut data = JsonMap::default();
    insert_data(&mut data);
    handlebars.render(name, &*data).map_err(Into::into)
}
