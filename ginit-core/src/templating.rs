use crate::{config::shared::Shared, util};
use bicycle::{
    handlebars::{Context, Handlebars, Helper, HelperResult, Output, RenderContext, RenderError},
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

pub fn init(config: Option<&Shared>) -> Bicycle {
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

pub fn template_pack(
    config: Option<&Shared>,
    plugin_path: Option<&Path>,
    name: &str,
) -> Option<PathBuf> {
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
    }
    // then we check the plugin
    if let Some(plugin_path) = plugin_path {
        if path.is_none() {
            path = try_path(plugin_path, name);
        }
    }
    // and then we check core templates
    if path.is_none() {
        path = try_path(env!("CARGO_MANIFEST_DIR"), name);
    }
    if path.is_none() {
        log::info!("template pack \"{}\" was never found", name);
    }
    path
}

#[macro_export]
macro_rules! template_pack {
    ($config:expr, $name:expr) => {
        $crate::templating::template_pack($config, Some(env!("CARGO_MANIFEST_DIR").as_ref()), $name)
    };
}
