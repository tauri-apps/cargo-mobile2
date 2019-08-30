use crate::config::{self, Config};
use bicycle::{
    handlebars::{Context, Handlebars, Helper, HelperResult, Output, RenderContext},
    Bicycle, EscapeFn, HelperDef, JsonMap,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

fn path<'a>(helper: &'a Helper) -> &'a str {
    helper
        .param(0)
        .and_then(|v| v.value().as_str())
        .unwrap_or("")
}

fn project_root<'a>(ctx: &'a Context) -> &'a str {
    ctx.data()
        .get("project_root")
        .expect("project root missing from template data")
        .as_str()
        .expect("project root wasn't a string")
}

fn prefix_path(
    helper: &Helper,
    _: &Handlebars,
    ctx: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    out.write(
        config::prefix_path(project_root(ctx), path(helper))
            .to_str()
            .expect("either project root or path contained invalid utf-8"),
    )
    .map_err(Into::into)
}

fn unprefix_path(
    helper: &Helper,
    _: &Handlebars,
    ctx: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    out.write(
        config::unprefix_path(project_root(ctx), path(helper))
            .expect("attempted to unprefix a path that wasn't in the project")
            .to_str()
            .expect("either project root or path contained invalid utf-8"),
    )
    .map_err(Into::into)
}

pub fn init_templating(config: Option<&Config>) -> Bicycle {
    Bicycle::new(
        EscapeFn::None,
        {
            let mut helpers = HashMap::<_, Box<dyn HelperDef>>::new();
            if config.is_some() {
                // don't mix these up or very bad things will happen to all of us
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

pub fn template_pack(config: Option<&Config>, name: &str) -> Option<PathBuf> {
    fn try_path(root: impl AsRef<Path>, name: &str) -> Option<PathBuf> {
        let path = root.as_ref().join("templates").join(name);
        log::info!("checking for template pack \"{}\" at {:?}", name, path);
        Some(path).filter(|path| {
            if path.exists() {
                log::info!("found template pack \"{}\" at {:?}", name, path);
                true
            } else {
                false
            }
        })
    }

    let mut path = None;
    if let Some(config) = config {
        // first we check the user's project
        path = try_path(config.project_root(), name);
        // then we check rust-lib
        if path.is_none() {
            path = try_path(config.app_root().join("rust-lib"), name);
        }
    }
    // and then we check our internal/bundled templates
    if path.is_none() {
        path = try_path(env!("CARGO_MANIFEST_DIR"), name);
    }
    if path.is_none() {
        log::info!("template pack \"{}\" was never found", name);
    }
    path
}
