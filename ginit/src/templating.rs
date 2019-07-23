use crate::config::Config;
use bicycle::{
    handlebars::{Context, Handlebars, Helper, HelperResult, Output, RenderContext},
    Bicycle, EscapeFn, HelperDef, JsonMap,
};
use std::{collections::HashMap, path::Path};

fn prefix_path(
    h: &Helper,
    _: &Handlebars,
    ctx: &Context,
    _: &mut RenderContext,
    out: &mut Output,
) -> HelperResult {
    let path = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let project_root = ctx
        .data()
        .get("project_root")
        .expect("project root missing from template data")
        .as_str()
        .expect("project root wasn't a string");
    let prefixed = Path::new(project_root).join(path);
    out.write(
        prefixed
            .to_str()
            .expect("either project root or path contained invalid utf-8"),
    )?;
    Ok(())
}

fn unprefix_path(
    h: &Helper,
    _: &Handlebars,
    ctx: &Context,
    _: &mut RenderContext,
    out: &mut Output,
) -> HelperResult {
    let path = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let project_root = ctx
        .data()
        .get("project_root")
        .expect("project root missing from template data")
        .as_str()
        .expect("project root wasn't a string");
    let unprefixed = Path::new(path)
        .strip_prefix(project_root)
        .expect("attempted to unprefix a path that wasn't in the project")
        .join(path);
    out.write(
        unprefixed
            .to_str()
            .expect("either project root or path contained invalid utf-8"),
    )?;
    Ok(())
}

pub fn init_templating(config: Option<&Config>) -> Bicycle {
    Bicycle::new(
        EscapeFn::None,
        {
            let mut helpers = HashMap::<_, Box<dyn HelperDef>>::new();
            if config.is_some() {
                helpers.insert("prefix_path", Box::new(prefix_path));
                helpers.insert("unprefix_path", Box::new(unprefix_path));
            }
            helpers
        },
        {
            let mut map = JsonMap::default();
            map.insert("tool_name", &*crate::NAME);
            if let Some(config) = config {
                map.insert("project_root", config.project_root());
            }
            map
        },
    )
}
