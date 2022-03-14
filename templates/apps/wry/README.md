# wry

## Android

From my setup, I also need to add `abiFilters += listOf("arm64-v8a")` under `create("arm")` branch in `:app`'s '`build.gradle.kts`.

This is probably different from users env, so I didn't add to the script.

## iOS

Must run Xcode on rosetta. Goto Application > Right Click Xcode > Get Info > Open in Rosetta.

If you are using M1, you will have to run `cargo build --target x86_64-apple-ios` instead of `cargo apple build` if you want to run in simulator.

Otherwise, it's all `cargo apple run` when running in actual device.
