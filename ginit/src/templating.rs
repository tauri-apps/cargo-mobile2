use ginit_core::{
    bicycle::{
        handlebars::{
            Context, Handlebars, Helper, HelperResult, Output, RenderContext, RenderError,
        },
        Bicycle, EscapeFn, HelperDef, JsonMap,
    },
    config::SharedConfig,
    util,
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

fn project_root<'a>(ctx: &'a Context) -> Result<&'a str, RenderError> {
    let project_root = ctx
        .data()
        .get("project-root")
        .ok_or_else(|| RenderError::new("`project-root` missing from template data."))?;
    project_root
        .as_str()
        .ok_or_else(|| RenderError::new("`project-root` contained invalid UTF-8."))
}

fn prefix_path(
    helper: &Helper,
    _: &Handlebars,
    ctx: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    out.write(
        util::prefix_path(project_root(ctx)?, path(helper))
            .to_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Either the `project-root` or the specified path contained invalid UTF-8.",
                )
            })?,
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
        util::unprefix_path(project_root(ctx)?, path(helper))
            .map_err(|_| {
                RenderError::new("Attempted to unprefix a path that wasn't in the project.")
            })?
            .to_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Either the `project-root` or the specified path contained invalid UTF-8.",
                )
            })?,
    )
    .map_err(Into::into)
}

pub fn init(config: Option<&SharedConfig>) -> Bicycle {
    Bicycle::new(
        EscapeFn::None,
        {
            let mut helpers = HashMap::<_, Box<dyn HelperDef>>::new();
            if config.is_some() {
                // don't mix these up or very bad things will happen to all of us
                helpers.insert("prefix-path", Box::new(prefix_path));
                helpers.insert("unprefix-path", Box::new(unprefix_path));
            }
            helpers
        },
        {
            let mut map = JsonMap::default();
            map.insert("tool-name", &*crate::NAME);
            if let Some(config) = config {
                map.insert("project-root", config.project_root());
            }
            map
        },
    )
}
