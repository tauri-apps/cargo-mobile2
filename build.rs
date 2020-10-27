use std::path::{Path, PathBuf};

fn main() {
    let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap();
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let home_dir = home::home_dir().expect("failed to get user's home dir");
    let bike = bicycle::Bicycle::default();
    for rel in ["platforms", "apps"]
        .iter()
        .map(|prefix| Path::new("templates").join(prefix))
    {
        let src = manifest_dir.join(&rel);
        println!("cargo:rerun-if-changed={}", src.display());
        let dest = home_dir.join(format!(".{}", pkg_name)).join(rel);
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
}
