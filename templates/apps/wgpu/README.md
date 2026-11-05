# wgpu-app

This is just the [`wgpu-rs` triangle example](https://github.com/gfx-rs/wgpu-rs/blob/v0.6/examples/hello-triangle/main.rs) with a handful of small changes, which I'll outline below:

- Render a red triangle instead of just clearing the screen green.
- Add the entry points `android_main` for android
- Annotate `start_app` as extern, so that i can be used via FFI on iOS
- Add logging via [`simple_logger`](https://crates.io/crates/simple_logger) and [`android_logger`](https://crates.io/crates/android_logger) 
- Reduce the limit `max_inter_stage_shader_variables` from 16 to 15 as a tempory fix for [this issue](https://github.com/gfx-rs/wgpu/issues/9386 is fixed
)

To run this on desktop, just do `cargo run` like normal! For mobile, use `cargo android run` and `cargo apple run` respectively (or use `cargo android open`/`cargo apple open` to open in Android Studio and Xcode respectively).
