# bevy-demo

This is just the [Bevy breakout example](https://github.com/bevyengine/bevy/blob/main/examples/games/breakout.rs) with a `#[mobile_entry_point]` attribute on `main`, which generates all the boilerplate `extern` functions for mobile.

To run this on desktop, just do `cargo run` like normal! For mobile, use `cargo android run` and `cargo apple run` respectively (or use `cargo android open` and `cargo apple open` to open in Android Studio and Xcode respectively).

Note that we've deliberately omitted the font used by the demo, since if the font loads successfully, it seems to trigger a text rendering crash in bevy.
