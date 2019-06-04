use derive_more::From;
use std::{ffi::OsStr, fs::File, io::{self, Write}, path::Path};

#[derive(Debug, From)]
pub enum RenderToFileError {
    RenderError(super::RenderError),
    FileCreationError(io::Error),
    WriteError(io::Error),
}

pub fn render_to_file(
    src: &Path,
    dest: &Path,
    insert_data: impl FnOnce(&mut super::JsonMap),
) -> Result<(), RenderToFileError> {
    let rendered = super::render_file(
        src.file_name().and_then(OsStr::to_str).unwrap(),
        src,
        insert_data,
    )?;
    let mut file = File::create(dest)
        .map_err(RenderToFileError::FileCreationError)?;
    file.write_all(rendered.as_bytes())
        .map_err(RenderToFileError::WriteError)
}
