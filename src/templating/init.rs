use crate::{
    config::{app, Config},
    util::{self, Git},
};
use bicycle::{
    handlebars::{
        self, Context, Handlebars, Helper, HelperResult, Output, RenderContext, RenderError,
    },
    Bicycle, EscapeFn, HelperDef, JsonMap,
};
use std::collections::HashMap;

fn get_str<'a>(helper: &'a Helper) -> &'a str {
    helper
        .param(0)
        .and_then(|v| v.value().as_str())
        .unwrap_or_else(|| "")
}

fn get_str_array<'a>(
    helper: &'a Helper,
    formatter: impl Fn(&str) -> String,
) -> Option<Vec<String>> {
    helper.param(0).and_then(|v| {
        v.value().as_array().and_then(|arr| {
            arr.iter()
                .map(|val| val.as_str().map(|s| formatter(s)))
                .collect()
        })
    })
}

fn html_escape(
    helper: &Helper,
    _: &Handlebars,
    _ctx: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    out.write(&handlebars::html_escape(get_str(helper)))
        .map_err(Into::into)
}

fn join(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    out.write(
        &get_str_array(helper, |s| format!("{}", s))
            .ok_or_else(|| RenderError::new("`join` helper wasn't given an array"))?
            .join(", "),
    )
    .map_err(Into::into)
}

fn quote_and_join(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    out.write(
        &get_str_array(helper, |s| format!("{:?}", s))
            .ok_or_else(|| RenderError::new("`quote-and-join` helper wasn't given an array"))?
            .join(", "),
    )
    .map_err(Into::into)
}

fn snake_case(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    use heck::SnekCase as _;
    out.write(&get_str(helper).to_snek_case())
        .map_err(Into::into)
}

fn reverse_domain(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    out.write(&util::reverse_domain(get_str(helper)))
        .map_err(Into::into)
}

fn app_root<'a>(ctx: &'a Context) -> Result<&'a str, RenderError> {
    let app_root = ctx
        .data()
        .get(app::KEY)
        .ok_or_else(|| RenderError::new("`app` missing from template data."))?
        .get("root-dir")
        .ok_or_else(|| RenderError::new("`app.root-dir` missing from template data."))?;
    app_root
        .as_str()
        .ok_or_else(|| RenderError::new("`app.root-dir` contained invalid UTF-8."))
}

fn prefix_path(
    helper: &Helper,
    _: &Handlebars,
    ctx: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    out.write(
        util::prefix_path(app_root(ctx)?, get_str(helper))
            .to_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Either the `app.root-dir` or the specified path contained invalid UTF-8.",
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
        util::unprefix_path(app_root(ctx)?, get_str(helper))
            .map_err(|_| {
                RenderError::new("Attempted to unprefix a path that wasn't in the app root dir.")
            })?
            .to_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Either the `app.root-dir` or the specified path contained invalid UTF-8.",
                )
            })?,
    )
    .map_err(Into::into)
}

fn detect_author() -> String {
    let git = Git::new(".".as_ref());
    let name_output = git.user_name().ok();
    let name = name_output.as_deref().unwrap_or_else(|| "Watashi");
    let email_output = git.user_email().ok();
    let email = email_output
        .as_deref()
        .unwrap_or_else(|| "watashi@example.com");
    format!("{} <{}>", name.trim(), email.trim())
}

pub fn init(config: Option<&Config>) -> Bicycle {
    Bicycle::new(
        EscapeFn::None,
        {
            let mut helpers = HashMap::<_, Box<dyn HelperDef + Send + Sync>>::new();
            helpers.insert("html-escape", Box::new(html_escape));
            helpers.insert("join", Box::new(join));
            helpers.insert("quote-and-join", Box::new(quote_and_join));
            helpers.insert("snake-case", Box::new(snake_case));
            helpers.insert("reverse-domain", Box::new(reverse_domain));
            if config.is_some() {
                // don't mix these up or very bad things will happen to all of us
                helpers.insert("prefix-path", Box::new(prefix_path));
                helpers.insert("unprefix-path", Box::new(unprefix_path));
            }
            helpers
        },
        {
            let mut map = JsonMap::default();
            if let Some(config) = config {
                map.insert(app::KEY, config.app());
                map.insert("author", detect_author());
                #[cfg(target_os = "macos")]
                map.insert(crate::apple::NAME, config.apple());
                map.insert(crate::android::NAME, config.android());
            }
            map
        },
    )
}
