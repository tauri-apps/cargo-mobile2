use std::path::PathBuf;

use crate::{
    open_harmony::{config::Config, env::Env},
    DuctExpressionExt,
};

pub fn install(config: &Config, env: &Env) -> std::io::Result<()> {
    log::info!("Installing packages with ohpm...");
    let mut ohpm_path = if cfg!(target_os = "macos") {
        PathBuf::from("/Applications/DevEco-Studio.app/Contents/tools/ohpm/bin/ohpm")
    } else if cfg!(target_os = "linux") {
        // OHOS_HOME is /path/to/sdk/default/openharmony, we want /path/to/ohpm/bin/ohpm so we must call parent() 3 times
        env.ohos_home()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("ohpm")
            .join("bin")
            .join("ohpm")
    } else {
        std::env::var("DEV_ECO_STUDIO_INSTALL_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:\\Program Files\\Huawei\\DevEco Studio"))
            .join("tools")
            .join("ohpm")
            .join("bin")
            .join("ohpm.bat")
    };
    if !ohpm_path.exists() {
        log::warn!(
            "ohpm not found in {}, expecting ohpm to be available in PATH...",
            ohpm_path.display()
        );
        ohpm_path = PathBuf::from("ohpm");
    }

    duct::cmd(ohpm_path, ["install"])
        .dir(config.project_dir())
        .dup_stdio()
        .start()?
        .wait()?;
    Ok(())
}
