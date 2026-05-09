pub fn enable_debug_layer() -> bool {
    std::env::var("DEBUG_LAYER").is_ok_and(|e| !(e == "0" || e == "false"))
}
