use std::path::PathBuf;

fn main() {
    let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap();
    let src = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("templates");
    let dest = dirs::home_dir()
        .expect("failed to get user's home dir")
        .join(format!(".{}/templates", pkg_name));
    let actions = bicycle::traverse(src, &dest, bicycle::no_transform, None)
        .expect("failed to traverse src templates dir");
    std::fs::remove_dir_all(dest).expect("failed to delete old templates");
    let bike = bicycle::Bicycle::default();
    bike.process_actions(
        actions.iter().inspect(|action| {
            if let bicycle::Action::CopyFile { src, .. } = action {
                println!("cargo:rerun-if-changed={}", src.display());
            }
        }),
        |_| (),
    )
    .expect("failed to process actions");
}
