use std::path::PathBuf;

fn main() {
    let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap();
    let src = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("templates");
    println!("cargo:rerun-if-changed={}", src.display());
    let dest = home::home_dir()
        .expect("failed to get user's home dir")
        .join(format!(".{}/templates", pkg_name));
    let actions = bicycle::traverse(&src, &dest, bicycle::no_transform, None)
        .expect("failed to traverse src templates dir");
    if dest.is_dir() {
        std::fs::remove_dir_all(&dest).expect("failed to delete old templates");
    }
    let bike = bicycle::Bicycle::default();
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
