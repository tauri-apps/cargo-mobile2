# wry

Currently works on Android only.

From my setup, I also need to add `abiFilters += listOf("arm64-v8a")` under `create("arm")` branch in `:app`'s '`build.gradle.kts`.
This is probably different from users env, so I didn't add to the script.
