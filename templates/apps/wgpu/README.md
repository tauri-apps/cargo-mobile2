# wgpu-app

This is just the [`wgpu-rs` triangle example](https://github.com/gfx-rs/wgpu-rs/blob/v0.6/examples/hello-triangle/main.rs) with a handful of small changes, which I'll outline below:

- Annotated `main` with `#[mobile_entry_point]`, which generates all the `extern` functions we need for mobile. Note that the name of the function doesn't actually matter for this.
- Changes conditionally compiled on Android:
  - Use `android_logger` instead of `wgpu-subscriber`
  - Use `Rgba8UnormSrgb` instead of `Bgra8UnormSrgb` (ideally, the supported format would be detected dynamically instead)
  - Use `std::thread::sleep` to shoddily workaround [`raw_window_handle` requirements](https://github.com/rust-windowing/winit/issues/1588)
  - Render directly upon `MainEventsCleared` instead of calling `request_redraw`, since [winit doesn't implement that method on Android yet](https://github.com/rust-windowing/winit/issues/1723)

To run this on desktop, just do `cargo run` like normal! For mobile, use `cargo android run` and `cargo apple run` respectively (or use `cargo android open`/`cargo apple open` to open in Android Studio and Xcode respectively).
