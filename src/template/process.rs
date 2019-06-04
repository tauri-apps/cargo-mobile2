use derive_more::From;
use log::info;
use std::{fs, io, iter, path::{Path, PathBuf}};

#[derive(Debug, From)]
pub enum ProcessingError {
    TraversalError(super::TraversalError<super::RenderError>),
    CreateDirectoryError(io::Error),
    CopyFileError(io::Error),
    WriteTemplateError(super::RenderToFileError),
}

pub fn render_path(
    path: &Path,
    insert_data: impl FnOnce(&mut super::JsonMap),
) -> Result<PathBuf, super::RenderError> {
    let path_str = path.to_str().unwrap();
    if path_str.contains("{{") {
        super::render_str(path_str, insert_data)
            .map(|rendered| Path::new(&rendered).to_owned())
    } else {
        Ok(path.to_owned())
    }
}

pub fn process_action(
    action: &super::Action,
    insert_data: impl Fn(&mut super::JsonMap),
) -> Result<(), ProcessingError> {
    info!("{:#?}", action);
    use super::Action::*;
    match action {
        CreateDirectory { dest } => {
            fs::create_dir_all(&dest)
                .map_err(ProcessingError::CreateDirectoryError)?;
        },
        CopyFile { src, dest } => {
            fs::copy(src, dest).map_err(ProcessingError::CopyFileError)?;
        },
        WriteTemplate { src, dest } => {
            super::render_to_file(&src, &dest, &insert_data)?;
        },
    }
    Ok(())
}

pub fn process_actions<'a>(
    actions: impl iter::Iterator<Item = &'a super::Action>,
    insert_data: impl Fn(&mut super::JsonMap),
) -> Result<(), ProcessingError> {
    for action in actions {
        process_action(action, &insert_data)?;
    }
    Ok(())
}

pub fn process(
    src: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    insert_data: impl Fn(&mut super::JsonMap),
) -> Result<(), ProcessingError> {
    super::traverse(src, dest, |path| render_path(path, &insert_data))
        .map_err(ProcessingError::TraversalError)
        .and_then(|actions| process_actions(actions.iter(), insert_data))
}
