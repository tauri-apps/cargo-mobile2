use spirv_builder::{ShaderPanicStrategy, SpirvBuilder, SpirvMetadata};
use std::path::PathBuf;

pub fn main() -> anyhow::Result<()> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let crate_path = [manifest_dir, "shaders"]
        .iter()
        .copied()
        .collect::<PathBuf>();

    let mut builder = SpirvBuilder::new(crate_path, "spirv-unknown-vulkan1.3");
    builder.build_script.defaults = true;
    builder.shader_panic_strategy = ShaderPanicStrategy::SilentExit;
    builder.spirv_metadata = SpirvMetadata::Full;

    let compile_result = builder.build()?;
    let spv_path = compile_result.module.unwrap_single();
    println!("cargo::rustc-env=SHADER_SPV_PATH={}", spv_path.display());
    Ok(())
}
