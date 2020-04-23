# Migrating from `cargo-ginit` to `cargo-mobile`

1. Install via `cargo install --force --git ssh://git@bitbucket.org/brainium/cargo-mobile.git`
2. Optionally, delete `cargo-ginit` with `rm $HOME/.cargo/bin/cargo-ginit`
3. In your projects, rename `ginit.toml` to `mobile.toml`
4. Make the following changes to `mobile.toml`:
   - Change any `snake_case` field names to `kebab-case`
   - Change `[ginit]` to `[app]`
   - Change `app-name` to `name`
   - Change `stylized-app-name` to `stylized-name`
   - Your config should look a lot like this (any field not shown isn't required):
    ```
    [app]
    name = "cool-game"
    stylized-name = "Cool Game"
    domain = "brainiumstudios.com"

    [apple]
    development-team = "7S85E2DAW8"
    ```
5. Add the following section to your `Cargo.toml` (put it anywhere you'd like):
    ```
    [package.metadata.cargo-android]
    features = ["vulkan"]

    [package.metadata.cargo-apple.ios]
    features = ["metal"]
    ```
6. Delete your `gen` folder
7. Finally, run `cargo mobile init`!

After that, you should be good to go!

## Some notable usage changes

- All the Android and iOS commands have been moved to separate `cargo android` and `cargo apple` bins, respectively. So, `cargo ginit android run` becomes `cargo android run`, `cargo ginit ios build` becomes `cargo apple build`, etc.
- `cargo mobile update` will replace the currently installed version of `cargo-mobile` with the latest version from master
- `cargo mobile open` will open your project in your default code editor (you can also do this automatically after `init` by using the `--open` flag)
- `cargo android open` will open your project in Android Studio
- `cargo apple open` will open your project in Xcode
