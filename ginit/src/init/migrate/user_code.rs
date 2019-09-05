use super::type_state::*;
use crate::config::Config;
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    marker::PhantomData,
};

static PROFILE: &'static str = r#"
[profile.dev]
opt-level = 0
debug = 2
rpath = false
lto = false
debug-assertions = true
codegen-units = 16
panic = "unwind"
incremental = true
overflow-checks = true

[profile.release]
opt-level = 3
debug = false
rpath = false
lto = "fat"
debug-assertions = false
codegen-units = 1
panic = "unwind"
incremental = false
overflow-checks = false
"#;

#[derive(Debug)]
pub struct UserCode<T> {
    _marker: PhantomData<T>,
}

impl UserCode<Legacy> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    pub fn move_to_root(self, config: &Config) -> io::Result<UserCode<Moved>> {
        let user_dir = config.app_root().join("rust").join(config.app_name());
        for entry in fs::read_dir(user_dir)? {
            let entry = entry?;
            let dest = if entry.file_name() == "build.rs" {
                config.app_root().join("gen/build.rs")
            } else {
                config.app_root().join(entry.file_name())
            };
            fs::rename(entry.path(), dest)?;
        }
        Ok(UserCode {
            _marker: PhantomData,
        })
    }
}

impl UserCode<Moved> {
    pub fn update_cargo_toml(self, config: &Config) -> io::Result<UserCode<Updated>> {
        let cargo_toml = config.app_root().join("Cargo.toml");
        let body = {
            let buf = {
                let mut file = File::open(&cargo_toml)?;
                let mut buf = String::new();
                file.read_to_string(&mut buf)?;
                buf
            };
            let lines = buf.lines().collect::<Vec<_>>();
            let new_length = lines.len() + 5;
            let mut new_lines = Vec::with_capacity(new_length);
            for line in lines {
                if line.starts_with("default") {
                    new_lines.push(line.replace("[]", "[\"code_reload\", \"metal\"]"));
                } else if line.starts_with("rust-lib") {
                    new_lines.push(line.replace("../lib", "rust-lib"));
                } else if line.starts_with("build") {
                    new_lines.push(line.replace("../lib/src/build", "rust-lib/src/build"));
                } else {
                    new_lines.push(line.to_owned());
                    let trimmed = line.trim();
                    if trimmed.starts_with("edition") {
                        new_lines.push(format!("build = \"gen/build.rs\""));
                    } else if trimmed.starts_with("crate-type") {
                        new_lines.push("\n[[bin]]".to_owned());
                        new_lines.push(format!("name = \"{}-desktop\"", config.app_name()));
                        new_lines.push("path = \"gen/bin/desktop.rs\"".to_owned());
                    }
                }
            }
            new_lines.push(PROFILE.to_owned());
            assert_eq!(new_lines.len(), new_length);
            new_lines.join("\n")
        };
        let mut file = File::create(&cargo_toml)?;
        file.write_all(body.as_bytes())?;
        Ok(UserCode {
            _marker: PhantomData,
        })
    }
}
