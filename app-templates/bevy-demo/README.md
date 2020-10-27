# bevy-demo

This is just the [Bevy breakout example](https://github.com/bevyengine/bevy/blob/master/examples/game/breakout.rs) with a `#[mobile_entry_point]` attribute on `main`, which generates all the boilerplate `extern` functions for mobile.

To run this on desktop, just do `cargo run` like normal! For mobile, use `cargo android run` and `cargo apple run` respectively (or use `cargo android open` and `cargo apple open` to open in Android Studio and Xcode respectively).
