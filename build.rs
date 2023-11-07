#[cfg(not(feature = "cli"))]
fn main() {}

#[cfg(feature = "cli")]
fn main() {
    use std::path::PathBuf;
    use std::process::Command;
    #[path = "src/bicycle/mod.rs"]
    mod bicycle;

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    use std::path::Path;

    let dir_name = concat!(".", env!("CARGO_PKG_NAME"));
    let install_dir = std::env::var("CARGO_HOME")
        .map(|p| PathBuf::from(p).join(dir_name))
        .unwrap_or_else(|_| {
            home::home_dir()
                .map(|home| home.join(".cargo").join(dir_name))
                .expect("failed to get user's home dir")
        });

    std::fs::create_dir_all(&install_dir).expect("failed to create install dir");

    // Copy version info
    match Command::new("git")
        .arg("-C")
        .arg(&manifest_dir)
        .args(["log", "-1", "--pretty=%s"])
        .output()
    {
        Ok(output) => {
            let msg = String::from_utf8_lossy(&output.stdout).to_string();
            if let Err(err) = std::fs::write(install_dir.join("commit"), msg) {
                println!(
                    "cargo:warning=failed to write current commit message: {}",
                    err
                )
            }
        }
        Err(err) => println!(
            "cargo:warning=failed to get current commit message: {}",
            err
        ),
    }

    // Copy templates
    let bike = bicycle::Bicycle::default();
    for rel in ["platforms", "apps"]
        .iter()
        .map(|prefix| Path::new("templates").join(prefix))
    {
        let src = manifest_dir.join(&rel);
        println!("cargo:rerun-if-changed={}", src.display());
        let dest = install_dir.join(rel);
        let actions = bicycle::traverse(&src, &dest, bicycle::no_transform, None)
            .expect("failed to traverse src templates dir");
        if dest.is_dir() {
            std::fs::remove_dir_all(&dest).expect("failed to delete old templates");
        }
        // bicycle creates directories for us, so we don't need to worry about
        // using `create_dir_all` or anything.
        bike.process_actions(
            actions.iter().inspect(|action| match action {
                bicycle::Action::CreateDirectory { dest: in_dest } => {
                    // This is sorta gross, but not really *that* gross, so...
                    let src = src.join(in_dest.strip_prefix(&dest).unwrap());
                    println!("cargo:rerun-if-changed={}", src.display());
                }
                bicycle::Action::CopyFile { src, .. } => {
                    println!("cargo:rerun-if-changed={}", src.display());
                }
                _ => (),
            }),
            |_| (),
        )
        .expect("failed to process actions");
    }

    #[cfg(windows)]
    {
        // Embed application manifest
        let resource_path = manifest_dir.join("cargo-mobile-manifest.rc");
        let manifest_path = manifest_dir.join("cargo-mobile.exe.manifest");
        println!("cargo:rerun-if-changed={}", resource_path.display());
        println!("cargo:rerun-if-changed={}", manifest_path.display());
        embed_resource::compile("cargo-mobile-manifest.rc", embed_resource::NONE);
    }
}
