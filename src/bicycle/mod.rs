//! `bicycle` is [`handlebars`] with wheels. 🚴🏽‍♀️

#![forbid(unsafe_code)]
#![allow(dead_code)]

mod json_map;
mod traverse;

pub use self::{json_map::*, traverse::*};
pub use handlebars::{self, HelperDef};

use handlebars::Handlebars;
use std::{
    fmt::{self, Debug},
    fs,
    io::{self, Read, Write},
    iter,
    path::{Component, Path, PathBuf, Prefix},
};
use thiserror::Error;

pub type CustomEscapeFn = &'static (dyn Fn(&str) -> String + 'static + Send + Sync);

/// Specifies how to escape template variables prior to rendering.
pub enum EscapeFn {
    /// The default setting. Doesn't change the variables at all.
    None,
    /// Escape anything that looks like HTML. This is recommended when rendering HTML templates with user-provided data.
    Html,
    /// Escape using a custom function.
    Custom(CustomEscapeFn),
}

impl Debug for EscapeFn {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmtr.pad(match self {
            Self::None => "None",
            Self::Html => "Html",
            Self::Custom(_) => "Custom(..)",
        })
    }
}

impl Default for EscapeFn {
    fn default() -> Self {
        Self::None
    }
}

impl From<CustomEscapeFn> for EscapeFn {
    fn from(custom: CustomEscapeFn) -> Self {
        Self::Custom(custom)
    }
}

/// An error encountered when rendering a template.
#[derive(Debug, Error)]
pub enum RenderingError {
    #[error("Failed to render template: {0}")]
    RenderingFailed(#[from] Box<handlebars::RenderError>),
}

/// An error encountered when processing an [`Action`].
#[derive(Debug, Error)]
pub enum ProcessingError {
    /// Failed to traverse files.
    #[error("Failed to traverse templates at {src:?}: {cause}")]
    Traversal {
        src: PathBuf,
        #[source]
        cause: TraversalError<RenderingError>,
    },
    /// Failed to create directory.
    #[error("Failed to create directory at {dest:?}: {cause}")]
    DirectoryCreation {
        dest: PathBuf,
        #[source]
        cause: io::Error,
    },
    /// Failed to copy file.
    #[error("Failed to copy file {src:?} to {dest:?}: {cause}")]
    FileCopy {
        src: PathBuf,
        dest: PathBuf,
        #[source]
        cause: io::Error,
    },
    /// Failed to open or read input file.
    #[error("Failed to read template at {src:?}: {cause}")]
    TemplateRead {
        src: PathBuf,
        #[source]
        cause: io::Error,
    },
    /// Failed to render template.
    #[error("Failed to render template at {src:?}: {cause}")]
    TemplateRender {
        src: PathBuf,
        #[source]
        cause: RenderingError,
    },
    /// Failed to create or write output file.
    #[error("Failed to write template from {src:?} to {dest:?}: {cause}")]
    TemplateWrite {
        src: PathBuf,
        dest: PathBuf,
        #[source]
        cause: io::Error,
    },
}

#[derive(Debug)]
pub struct Bicycle {
    handlebars: Handlebars<'static>,
    base_data: JsonMap,
}

impl Default for Bicycle {
    fn default() -> Self {
        Self::new(Default::default(), iter::empty(), Default::default())
    }
}

impl Bicycle {
    /// Creates a new `Bicycle` instance, using the provided arguments to
    /// configure the underlying [`handlebars::Handlebars`] instance.
    ///
    /// For info on `helpers`, consult the [`handlebars` docs](../handlebars/index.html#custom-helper).
    ///
    /// `base_data` is data that will be available for all invocations of all methods on this instance.
    ///
    /// # Examples
    /// ```
    /// use cargo_mobile2::bicycle::{
    ///     handlebars::{handlebars_helper, HelperDef},
    ///     Bicycle, EscapeFn, JsonMap,
    /// };
    /// use std::collections::HashMap;
    ///
    /// // An escape function that just replaces spaces with an angry emoji...
    /// fn spaces_make_me_very_mad(raw: &str) -> String {
    ///     raw.replace(' ', "😡")
    /// }
    ///
    /// // A helper to reverse strings.
    /// handlebars_helper!(reverse: |s: str|
    ///     // This doesn't correctly account for graphemes, so
    ///     // use a less naïve implementation for real apps.
    ///     s.chars().rev().collect::<String>()
    /// );
    ///
    /// // You could just as well use a [`Vec`] of tuples, or in this case,
    /// // [`std::iter::once`].
    /// let mut helpers = HashMap::<_, Box<dyn HelperDef + Send + Sync>>::new();
    /// helpers.insert("reverse", Box::new(reverse));
    ///
    /// let bike = Bicycle::new(
    ///     EscapeFn::Custom(&spaces_make_me_very_mad),
    ///     helpers,
    ///     JsonMap::default(),
    /// );
    /// ```
    pub fn new<'helper_name>(
        escape_fn: EscapeFn,
        helpers: impl iter::IntoIterator<
            Item = (
                &'helper_name str,
                Box<dyn HelperDef + Send + Sync + 'static>,
            ),
        >,
        base_data: JsonMap,
    ) -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        match escape_fn {
            EscapeFn::Custom(escape_fn) => handlebars.register_escape_fn(escape_fn),
            EscapeFn::None => handlebars.register_escape_fn(handlebars::no_escape),
            EscapeFn::Html => handlebars.register_escape_fn(handlebars::html_escape),
        }
        for (name, helper) in helpers {
            handlebars.register_helper(name, helper);
        }
        Self {
            handlebars,
            base_data,
        }
    }

    /// Renders a template.
    ///
    /// Use `insert_data` to define any variables needed for the template.
    ///
    /// # Examples
    /// ```
    /// use cargo_mobile2::bicycle::Bicycle;
    ///
    /// let bike = Bicycle::default();
    /// let rendered = bike.render("Hello {{name}}!", |map| {
    ///     map.insert("name", "Shinji");
    /// }).unwrap();
    /// assert_eq!(rendered, "Hello Shinji!");
    /// ```
    pub fn render(
        &self,
        template: &str,
        insert_data: impl FnOnce(&mut JsonMap),
    ) -> Result<String, RenderingError> {
        let mut data = self.base_data.clone();
        insert_data(&mut data);
        self.handlebars
            .render_template(template, &data.0)
            .map_err(Box::new)
            .map_err(Into::into)
    }

    /// Executes an [`Action`].
    ///
    /// - [`Action::CreateDirectory`] is executed with the same semantics as `mkdir -p`:
    ///   any missing parent directories are also created, and creation succeeds even if
    ///   the directory already exists. Failure results in a [`ProcessingError::DirectoryCreationFailed`].
    /// - [`Action::CopyFile`] is executed with the same semantics as `cp`:
    ///   if the destination file already exists, it will be overwritted with a copy of
    ///   the source file. Failure results in a [`ProcessingError::FileCopyFailed`].
    /// - [`Action::WriteTemplate`] is executed by reading the source file,
    ///   rendering the contents as a template (using `insert_data` to pass
    ///   any required values to the underlying [`Bicycle::render`] call),
    ///   and then finally writing the result to the destination file. The destination
    ///   file will be overwritten if it already exists. Failure for each step results
    ///   in [`ProcessingError::TemplateReadFailed`], [`ProcessingError::TemplateRenderFailed`],
    ///   and [`ProcessingError::TemplateWriteFailed`], respectively.
    pub fn process_action(
        &self,
        action: &Action,
        insert_data: impl Fn(&mut JsonMap),
    ) -> Result<(), ProcessingError> {
        log::info!("{:#?}", action);
        match action {
            Action::CreateDirectory { dest } => {
                fs::create_dir_all(dest).map_err(|cause| ProcessingError::DirectoryCreation {
                    dest: dest.clone(),
                    cause,
                })?;
            }
            Action::CopyFile { src, dest } => {
                fs::copy(src, dest).map_err(|cause| ProcessingError::FileCopy {
                    src: src.clone(),
                    dest: dest.clone(),
                    cause,
                })?;
            }
            Action::WriteTemplate { src, dest } => {
                let mut template = String::new();
                fs::File::open(src)
                    .and_then(|mut file| file.read_to_string(&mut template))
                    .map_err(|cause| ProcessingError::TemplateRead {
                        src: src.clone(),
                        cause,
                    })?;
                let rendered = self.render(&template, insert_data).map_err(|cause| {
                    ProcessingError::TemplateRender {
                        src: src.clone(),
                        cause,
                    }
                })?;
                fs::File::create(dest)
                    .and_then(|mut file| file.write_all(rendered.as_bytes()))
                    .map_err(|cause| ProcessingError::TemplateWrite {
                        src: src.clone(),
                        dest: dest.clone(),
                        cause,
                    })?;
            }
        }
        Ok(())
    }

    /// Iterates over `actions`, passing each item to [`Bicycle::process_action`].
    pub fn process_actions<'iter_item>(
        &self,
        actions: impl iter::Iterator<Item = &'iter_item Action>,
        insert_data: impl Fn(&mut JsonMap),
    ) -> Result<(), ProcessingError> {
        for action in actions {
            self.process_action(action, &insert_data)?;
        }
        Ok(())
    }

    /// A convenience method that calls [`traverse`](traverse()) and passes the
    /// output to [`Bicycle::process_actions`]. Uses [`Bicycle::transform_path`]
    /// as the `transform_path` argument and `DEFAULT_TEMPLATE_EXT` ("hbs") as
    /// the `template_ext` argument to [`traverse`](traverse()).
    pub fn process(
        &self,
        src: impl AsRef<Path>,
        dest: impl AsRef<Path>,
        insert_data: impl Fn(&mut JsonMap),
    ) -> Result<(), ProcessingError> {
        self.filter_and_process(src, dest, insert_data, |_| true)
    }

    /// A convenience method that does the same work as [`Bicycle::process`],
    /// but applies a filter predicate to each action prior to processing it.
    pub fn filter_and_process(
        &self,
        src: impl AsRef<Path>,
        dest: impl AsRef<Path>,
        insert_data: impl Fn(&mut JsonMap),
        mut filter: impl FnMut(&Action) -> bool,
    ) -> Result<(), ProcessingError> {
        let src = src.as_ref();
        traverse(
            src,
            dest,
            |path| self.transform_path(path, &insert_data),
            DEFAULT_TEMPLATE_EXT,
        )
        .map_err(|cause| ProcessingError::Traversal {
            src: src.to_owned(),
            cause,
        })
        .and_then(|actions| {
            self.process_actions(actions.iter().filter(|action| filter(action)), insert_data)
        })
    }

    /// Renders a path string itself as a template.
    /// Intended to be used as the `transform_path` argument to [`traverse`](traverse()).
    pub fn transform_path(
        &self,
        path: &Path,
        insert_data: impl FnOnce(&mut JsonMap),
    ) -> Result<PathBuf, RenderingError> {
        // On Windows, backslash is the path separator, and passing that
        // to handlebars, will make it think that "path\to\{{something}}"
        // is an escaped sequence and won't render correctly, so we need to
        // disassemble the path into its compoenents and build it backup
        // but use forward slash as a separator
        let path_str = dunce::simplified(path)
            .components()
            .map(|c| c.as_os_str().to_str().unwrap())
            .collect::<Vec<_>>()
            .join("/");
        // This is naïve, but optimistically isn't a problem in practice.
        if path_str.contains("{{") {
            self.render(&path_str, insert_data)
                .map(PathBuf::from)
                .map(|p| p.components().collect::<PathBuf>())
                .map(|p| {
                    if let Some(Component::Prefix(prefix)) = p.components().next() {
                        if let Prefix::Disk(_) = prefix.kind() {
                            return p
                                .to_str()
                                .map(|s| format!("\\\\?\\{}", s))
                                .map(PathBuf::from)
                                .unwrap_or_else(|| p);
                        }
                    }
                    p
                })
        } else {
            Ok(path.to_owned())
        }
    }
}
