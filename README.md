# cargo-mobile

*The answer to "how do I use Rust on iOS and Android?"*

cargo-mobile takes care of generating Xcode and Android Studio project files, building and running on device, generating project boilerplate, and a few other things!

Check out the [announcement post](https://dev.brainiumstudios.com/2020/11/24/cargo-mobile.html)!

## Status

Everything here works and is already used internally! However, this hasn't seen a lot of external use yet, so there could still be some rough edges.

**Building for iOS is broken on Rust 1.46.0, 1.47.0, and 1.48.0!**

Sorry for the inconvenience! This is resolved in Rust 1.49.0, so please update to that if you haven't already:

```bash
rustup update stable
rustup default stable
```

## Installation

The build will probably take a bit, so feel free to go get a snack or something.

```bash
cargo install --git https://github.com/BrainiumLLC/cargo-mobile
```

cargo-mobile is currently supported on macOS and Linux. Note that it's not possible to target iOS on platforms other than macOS! You'll still get to target Android either way.

A PR adding Windows support would be hugely appreciated!

You'll need to have Xcode and the Android SDK/NDK installed. Some of this will ideally be automated in the future, or at least we'll provide a helpful guide and diagnostics.

Whenever you want to update:

```bash
cargo mobile update
```

## Usage

To start a new project, all you need to do is make a directory with a cute name, `cd` into it, and then run this command:

```bash
cargo mobile init
```

After some straightforward prompts, you'll be asked to select a template pack. Template packs are used to generate project boilerplate, i.e. using the `bevy` template pack gives you a minimal [Bevy](https://bevyengine.org/) project that runs out-of-the-box on desktop and mobile.

| name      | info                                                                                                                              |
| --------- | --------------------------------------------------------------------------------------------------------------------------------- |
| bevy      | Minimal Bevy project derived from [sprite](https://github.com/bevyengine/bevy/blob/master/examples/2d/sprite.rs) example          |
| bevy-demo | Bevy [breakout](https://github.com/bevyengine/bevy/blob/master/examples/game/breakout.rs) demo                                    |
| wgpu      | Minimal wgpu project derived from [hello-triangle](https://github.com/gfx-rs/wgpu-rs/tree/master/examples/hello-triangle) example |
| winit     | Minimal winit project derived from [window](https://github.com/rust-windowing/winit/tree/master/examples/window) example          |

**Template pack contribution is encouraged**; we'd love to have very nice template packs for Bevy, Amethyst, and whatever else people find helpful! We'll write up a guide for template pack creation soon, but in the mean time, the existing ones are a great reference point. Any template pack placed into `~./cargo-mobile/templates/apps/` will appear as an option in `cargo mobile init`.

Once you've generated your project, you can run `cargo run` as usual to run your app on desktop. However, now you can also do `cargo apple run` and `cargo android run` to run on connected iOS and Android devices respectively!

If you prefer to work in the usual IDEs, you can use `cargo apple open` and `cargo android open` to open your project in Xcode and Android Studio respectively.

For more commands, run `cargo mobile`, `cargo apple`, or `cargo android` to see help information.

### Android

**Building for Android is broken on NDK >= 23**

`cargo android run` will build, install and run the app and follows device logs emitted by the app.

By default, warn and error logs are displayed. Additional logging of increasing verbosity can be shown by use of the `-v` or `-vv` options. These also provide more verbose logging for the build and install steps.

For fine-grained control of logging, use the `--filter` (or `-f`) option, which takes an Android log level, such as `debug`. This option overrides
the default device logging level set by `-v` or `-vv`.

If using the `android_logger` crate to handle Rust log messages, `trace` logs from Rust are mapped to `verbose` logs in Android.
