use freedesktop_entry_parser::{parse_entry, Entry as FreeDesktopEntry};
use once_cell_regex::{byte_regex, exports::regex::bytes::Regex};
use std::{
    env,
    ffi::{OsStr, OsString},
    io,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

// Detects which .desktop file contains the data on how to handle a given
// mime type (like: "with which program do I open a text/rust file?")
pub fn query_mime_entry(mime_type: &str) -> Option<PathBuf> {
    duct::cmd("xdg-mime", ["query", "default", mime_type])
        .read()
        .map(|out_str| {
            log::debug!("query_mime_entry got output {:?}", out_str);
            if !out_str.is_empty() {
                Some(PathBuf::from(out_str.trim()))
            } else {
                None
            }
        })
        .ok()?
}

// Returns the first entry on that directory whose filename is equal to target.
//
// This spec is what makes me believe the search is recursive:
// https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html
// This other one does not give that idea:
// https://specifications.freedesktop.org/menu-spec/latest/ar01s02.html
pub fn find_entry_in_dir(dir_path: &Path, target: &Path) -> std::io::Result<Option<PathBuf>> {
    for entry in dir_path.read_dir()?.flatten() {
        // If it is a file with that same _filename_ (not full path)
        if entry.path().is_file() && entry.file_name() == target {
            return Ok(Some(entry.path()));
        } else if entry.path().is_dir() {
            // I think if there are any dirs on that directory we have to
            // recursively search on them
            if let Some(result) = find_entry_in_dir(&entry.path(), target)? {
                return Ok(Some(result));
            }
        }
    }
    Ok(None)
}

pub fn parse(entry: impl AsRef<Path>) -> io::Result<FreeDesktopEntry> {
    parse_entry(entry.as_ref())
}

/// Returns the first FreeDesktop XDG .desktop entry, found inside `dir_path`, when the
/// "Name" atribute of that entry is `app_name`.
///
/// The return value is actually a tuple containing the entry itself, and the path at which
/// it was found.
pub fn find_entry_by_app_name(
    dir_path: &Path,
    app_name: &OsStr,
) -> Option<(FreeDesktopEntry, PathBuf)> {
    for entry in dir_path.read_dir().ok()?.filter_map(Result::ok) {
        let entry_path = entry.path();
        // If it is a file we open it
        if entry_path.is_file() {
            if let Ok(parsed) = parse_entry(&entry_path) {
                if parsed
                    .section("Desktop Entry")
                    .attr("Name")
                    .map(str::as_ref)
                    == Some(app_name)
                {
                    return Some((parsed, entry_path));
                }
            }
        } else if entry.path().is_dir() {
            // Recursively keep searching if it is a directory
            if let Some(result) = find_entry_by_app_name(&entry_path, app_name) {
                return Some(result);
            }
        }
    }
    None
}

fn replace_on_pattern(
    text: impl AsRef<OsStr>,
    replace_by: impl AsRef<OsStr>,
    regex: &Regex,
) -> OsString {
    let text = text.as_ref();
    let replace_by = replace_by.as_ref();

    // Vec<u8> is easier to deal with than OsString, and on unix they're pretty much
    // the same thing (OsStringExt).
    let mut result_text = Vec::new();
    let mut last_index_read = 0;

    for mat in regex.find_iter(text.as_bytes()) {
        let start = mat.start();
        let end = mat.end();

        // We put the values from the last index we read, to the start of the matching regex
        result_text.extend_from_slice(&text.as_bytes()[last_index_read..start]);

        // We put the part we want to replace the match with
        result_text.extend_from_slice(replace_by.as_bytes());

        // Then we jump the last index to the end of the regex, ignoring the part we matched
        last_index_read = end;
    }
    // At the end of the loop, put the rest of the string
    result_text.extend_from_slice(&text.as_bytes()[last_index_read..]);

    OsString::from_vec(result_text)
}

fn parse_quoted_text(
    text: &OsStr,
    argument: &OsStr,
    icon: Option<&OsStr>,
    desktop_entry_path: Option<&Path>,
) -> OsString {
    // We parse the escape character (\) again on the quoted text
    let mut result = Vec::new();
    let mut escaping = false;
    for &c in text.as_bytes() {
        if escaping {
            // If escaping, then pass whatever char c is, then stop escaping
            result.push(c);
            escaping = false;
        } else {
            // If not escaping, check for whether c is escape ('\'), going into escaping
            // mode if yes (dropping c). Otherwise just pass the c char.
            if c == b'\\' {
                escaping = true;
            } else {
                result.push(c);
            }
        }
    }
    let result = OsString::from_vec(result);

    // Now we do the unquoted part
    parse_unquoted_text(&result, argument, icon, desktop_entry_path)
}

fn parse_unquoted_text(
    text: &OsStr,
    argument: &OsStr,
    icon: Option<&OsStr>,
    desktop_entry_path: Option<&Path>,
) -> OsString {
    // We parse the arguments
    // We only have one file path (not an URL). Any instance of these ones
    // needs to be replaced by the file path in this particular case.
    let arg_re = byte_regex!(r"%u|%U|%f|%F");
    let result = replace_on_pattern(text, argument, arg_re);

    // Then the other flags
    let icon_replace = icon.unwrap_or_else(|| "".as_ref());
    let result = replace_on_pattern(result, icon_replace, byte_regex!("%i"));

    let desktop_entry_replace = desktop_entry_path.unwrap_or_else(|| "".as_ref());
    let result = replace_on_pattern(result, desktop_entry_replace, byte_regex!("%k"));

    // The other % flags are deprecated so we clear them, except double percentage
    // The spec from freedesktop does not even list what they should mean
    let result = replace_on_pattern(result, "", byte_regex!(r"%[^%]"));

    // Of course, the double percentage maps to percentage
    replace_on_pattern(result, "%", byte_regex!("%%"))
}

// The exec field of the FreeDesktop entry may contain some flags that need to
// be replaced by parameters or even other stuff. I am trying to implement it
// all this time.
//
// This function kind of became a monster
pub fn parse_command(
    command: &OsStr,
    argument: &OsStr,
    icon: Option<&OsStr>,
    desktop_entry_path: Option<&Path>,
) -> Vec<OsString> {
    log::debug!(
        "Parsing XDG Exec command {:?}, with argument {:?}",
        command,
        argument
    );

    // let command_name_re = byte_regex!(r#"^[^ \t"]+|"[^ \t]+""#);
    let mut escape_char = false;
    let mut reading_quoted = false;
    let mut reading_singlequoted = false;

    let mut parsed_command_parts = Vec::new();
    let mut text_atom = Vec::new();

    // I think doing it like this, although a bit big, is the clearest way to follow the scheme described on the
    // specification:
    // https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#exec-variables
    // We even need to escape backslash TWICE when we're inside quotes, as it is written:
    // "Likewise, a literal dollar sign in a quoted argument in a desktop entry file is unambiguously represented with ("\\$")."
    //
    // The idea is to separate and unquote the arguments first, then do some regex replacements on the arguments individually.
    // The spec itself says "Implementations must undo quoting before expanding field codes..."
    for &c in command.as_bytes() {
        // If we are escaping something we will just let it pass
        if escape_char {
            text_atom.push(c);
            escape_char = false;
        // Otherwise, we have to pay special attention to backslash
        } else if c == b'\\' {
            // If we see a backslash and are not escaping anything we will not "read" the
            // backslash, and instead escape the next char.
            escape_char = true;
        // If we're reading a quoted argument ("like this")
        } else if reading_quoted {
            if c != b'"' {
                text_atom.push(c);
            } else {
                // When we find another ", we collected a text atom
                // If there is text we store it
                if !text_atom.is_empty() {
                    let text_atom_string = parse_quoted_text(
                        OsStr::from_bytes(&text_atom),
                        argument,
                        icon,
                        desktop_entry_path,
                    );
                    parsed_command_parts.push(text_atom_string);
                    text_atom.clear();
                }
                // And the quoted ended
                reading_quoted = false;
            }
        // If we're reading a singly quoted argument ('like this')
        } else if reading_singlequoted {
            // Same thing but for '
            if c != b'\'' {
                text_atom.push(c);
            } else {
                // When we find another ', we collected a text atom
                // If there is text we store it
                if !text_atom.is_empty() {
                    let text_atom_string = parse_quoted_text(
                        OsStr::from_bytes(&text_atom),
                        argument,
                        icon,
                        desktop_entry_path,
                    );
                    parsed_command_parts.push(text_atom_string);
                    text_atom.clear();
                }
                // And the quoting ended
                reading_singlequoted = false;
            }
        // If not quoting, or scaping, then space is a text atom separator
        } else if [b' ', b'\t', b'\n'].contains(&c) {
            // If there is text we store it
            if !text_atom.is_empty() {
                let text_atom_string = parse_unquoted_text(
                    OsStr::from_bytes(&text_atom),
                    argument,
                    icon,
                    desktop_entry_path,
                );
                parsed_command_parts.push(text_atom_string);
                text_atom.clear();
            }
        // If a non whitespace, nor backslash character, when we're neither escaping nor in quotes, then...
        } else {
            match c {
                b'"' => reading_quoted = true,
                b'\'' => reading_singlequoted = true,
                anything_else => text_atom.push(anything_else),
            }
        }
    } // End of iteration over the command's bytes

    // At the end of the loop we flush whatever was being accumulated to the command parts
    if !text_atom.is_empty() {
        // If the value was well formed, quoted strings end on a quote character, and
        // not on EOF, so this should be unquoted.
        let text_atom_string = parse_unquoted_text(
            OsStr::from_bytes(&text_atom),
            argument,
            icon,
            desktop_entry_path,
        );
        parsed_command_parts.push(text_atom_string);
        text_atom.clear();
    }

    log::debug!(
        "XDG parsed command {:?} to {:?}",
        command,
        parsed_command_parts
    );
    parsed_command_parts
}

// Returns a vector of all the relevant xdg desktop application entries
// Check out:
// https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html
// https://wiki.archlinux.org/index.php/XDG_Base_Directory
// That explains the default values and the relevant variables.
pub fn get_xdg_data_dirs() -> Vec<PathBuf> {
    let mut result = Vec::new();

    if let Ok(home) = crate::util::home_dir() {
        let xdg_data_home = env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".local/share")); // The default
        result.push(xdg_data_home);
    }

    if let Ok(var) = env::var("XDG_DATA_DIRS") {
        let entries = var.split(':').map(PathBuf::from);
        result.extend(entries);
    } else {
        // These are the default ones we'll use in case the var is not set
        result.push(PathBuf::from("/usr/local/share"));
        result.push(PathBuf::from("/usr/share"));
    };

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_simple() {
        assert_eq!(
            parse_command(
                r#"simple.sh %u"#.as_ref(),
                "~/myfolder/src".as_ref(),
                None,
                None,
            ),
            ["simple.sh", "~/myfolder/src"]
        );
    }

    #[test]
    fn parse_command_simple_quote_test() {
        assert_eq!(
            parse_command(
                r#"simple.sh "%u" "single 'quotes' inside" 'double "quotes" inside' \"not quoted\""#.as_ref(),
                "~/my folder/src".as_ref(),
                None,
                None,
            ),
            ["simple.sh", "~/my folder/src", "single 'quotes' inside", r#"double "quotes" inside"#, "\"not", "quoted\""]
        );
    }

    #[test]
    fn parse_command_escape_test() {
        assert_eq!(
            parse_command(
                r#"cargo run -- these are separated these\ are\ together "This is a dollar sign: \\$" %u \\ \$ \`"#.as_ref(),
                "filename.txt".as_ref(),
                None,
                None,
            ),
            ["cargo", "run", "--", "these", "are", "separated", "these are together", "This is a dollar sign: $", "filename.txt", r"\", "$", "`"]
        );
    }

    #[test]
    fn parse_command_complex_test() {
        assert_eq!(
            parse_command(
                r#"test_command --flag %u --another "thing \\\\" %i %% %k My\ Work\ Place"#
                    .as_ref(),
                "/my/file/folder/file.rs".as_ref(),
                Some("/foo/bar/something/myicon.xpg".as_ref()),
                Some("/foo/bar/applications/test.desktop".as_ref()),
            ),
            [
                "test_command",
                "--flag",
                "/my/file/folder/file.rs",
                "--another",
                r"thing \",
                "/foo/bar/something/myicon.xpg",
                "%",
                "/foo/bar/applications/test.desktop",
                "My Work Place"
            ]
        );
    }
}
