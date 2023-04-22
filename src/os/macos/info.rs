use crate::{os::Info, util};
use once_cell_regex::regex;

pub fn check() -> Result<Info, util::RunAndSearchError> {
    util::run_and_search(
        &mut duct::cmd("system_profiler", ["SPSoftwareDataType"]),
        regex!(r"macOS (?P<version>.*)"),
        |_output, caps| caps.name("version").unwrap().as_str().to_owned(),
    )
    .map(|version| Info {
        name: "macOS".to_owned(),
        version,
    })
}
