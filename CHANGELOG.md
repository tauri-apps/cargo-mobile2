# Changelog

## \[0.5.1]

- [`e1bf612`](https://github.com/tauri-apps/tauri-mobile/commit/e1bf612e6f2cf1f1ff21401d01129946dffe9abb)([#162](https://github.com/tauri-apps/tauri-mobile/pull/162)) Update the path to `libc++_shared.so` for NDK versions 22 and above.
- [`7b46c30`](https://github.com/tauri-apps/tauri-mobile/commit/7b46c30764e1c410e25f519a95192346f41a9e3b)([#169](https://github.com/tauri-apps/tauri-mobile/pull/169)) Added the `native-tls` (enabled by default) and `rustls` features to allow compiling without native-tls.

## \[0.5.0]

- [`c2abaf5`](https://github.com/tauri-apps/tauri-mobile/commit/c2abaf54135bf65b1165a38d3b1d84e8d57f5d6c)([#148](https://github.com/tauri-apps/tauri-mobile/pull/148)) Detach launched processes for CLI `open` commands on macOS and Windwos.
- [`489d812`](https://github.com/tauri-apps/tauri-mobile/commit/489d812c134efdc80fb08c70b3936a8395fd4216)([#149](https://github.com/tauri-apps/tauri-mobile/pull/149)) Fix several commands fail because stdout isn't captured.
- [`1245540`](https://github.com/tauri-apps/tauri-mobile/commit/12455407f447ea7becfce19e16fcdca6b4f843f8)([#153](https://github.com/tauri-apps/tauri-mobile/pull/153)) Update android template to gradle 8.0
- [`8f6c122`](https://github.com/tauri-apps/tauri-mobile/commit/8f6c122f886d69b13df045cfc593b8d510b02dc7)([#152](https://github.com/tauri-apps/tauri-mobile/pull/152)) Change CLI template directory to `$CARGO_HOME/.tauri-mobile` instead of `$HOME/.tauri-mobile`.

## \[0.4.0]

- Use `duct` to run the ADB commands.
  - [8caa30c](https://github.com/tauri-apps/tauri-mobile/commit/8caa30c8fc3369b94f5da18c246bf945e32c8a4f) fix(android): use duct to run the ADB commands ([#134](https://github.com/tauri-apps/tauri-mobile/pull/134)) on 2023-04-15
- **Breaking** Replace `bossy` with `duct` across the crate. bossy has two ways to create commands, impure and pure. The pure version won't inherit env variables. This causes child processes won't get the env varialbes and results in issues like openssl cross compilation.
  - [6ee75fb](https://github.com/tauri-apps/tauri-mobile/commit/6ee75fbb10a17624e3a3ec64ab04dad2928ef9ed) refactor: replace bossy with duct ([#143](https://github.com/tauri-apps/tauri-mobile/pull/143)) on 2023-04-22
- Return `duct::Handle` in `apple::Device::run` to keep compatibility with Android.
  - [84311da](https://github.com/tauri-apps/tauri-mobile/commit/84311da4aec6f30c9158f60caea285e61ffef32f) refactor(apple): use duct for Device::run commands ([#137](https://github.com/tauri-apps/tauri-mobile/pull/137)) on 2023-04-15
- Update `wry` template to `wry@0.28`
  - [0b9580f](https://github.com/tauri-apps/tauri-mobile/commit/0b9580ff702d57ba5d3095de95ce77e4539b7874) chore: update wry template ([#141](https://github.com/tauri-apps/tauri-mobile/pull/141)) on 2023-04-17

## \[0.3.0]

- This change manually instructs Java and Kotlin to use/generate code for the same JVM target.
  - [fee2f07](https://github.com/tauri-apps/tauri-mobile/commit/fee2f07c41d43e9a6a801a05c879b5c7e211d837) Update build.gradle.kts.hbs to fix Kotlin incorrect version usage ([#122](https://github.com/tauri-apps/tauri-mobile/pull/122)) on 2023-03-25
- Remove `libgcc` redirect to `libunwind` workaround for NDK 23 and higher
  - [51a5072](https://github.com/tauri-apps/tauri-mobile/commit/51a5072be644d60e4854c60979b9ab425e7edf0e) refactor: remove libunwind workaround for ndk ([#125](https://github.com/tauri-apps/tauri-mobile/pull/125)) on 2023-03-29
- Use signed apks if signing is configured in the gradle project.
  - [2d37899](https://github.com/tauri-apps/tauri-mobile/commit/2d37899deaac6afe46bf710ef44719d10e398983) feat: handle signed apks ([#124](https://github.com/tauri-apps/tauri-mobile/pull/124)) on 2023-03-29
- Build only specified rust targets for `cargo android apk build` and `cargo android aab build` instead of all.
  - [ecb56d8](https://github.com/tauri-apps/tauri-mobile/commit/ecb56d8066d799488f4d200f865138fb2c22c6ca) fix: only build specified rust targets for aab/apk build ([#128](https://github.com/tauri-apps/tauri-mobile/pull/128)) on 2023-04-04

## \[0.2.5]

- Add `start_detached` method to start emulators.
  - [ce1ba93](https://github.com/tauri-apps/tauri-mobile/commit/ce1ba93cd1865f6d5742eaa2d15ff776819e366d) feat: add `start_detached` to emulators ([#114](https://github.com/tauri-apps/tauri-mobile/pull/114)) on 2023-03-16
- Fallback to `gradlew` or `gradle` from `PATH` if the one inside the generated template doesn't exist.
  - [442f0d2](https://github.com/tauri-apps/tauri-mobile/commit/442f0d2c7328930db61058a55706d22e6a401c16) fix: fallback to gradlew from PATH if the template doesn't have one ([#111](https://github.com/tauri-apps/tauri-mobile/pull/111)) on 2023-03-07
  - [c18c21e](https://github.com/tauri-apps/tauri-mobile/commit/c18c21e8f1edea04b46f22070098cce71efa0ad4) fix: fallback to `gradle` ([#113](https://github.com/tauri-apps/tauri-mobile/pull/113)) on 2023-03-16
- Use correct lib name in xcode project.
  - [2983144](https://github.com/tauri-apps/tauri-mobile/commit/298314485ed0f0acb1cb423b812275cd8dcafc0f) fix: use correct lib name in xcode project ([#110](https://github.com/tauri-apps/tauri-mobile/pull/110)) on 2023-03-02
- Add xcode script back and skip it when building simulator target.
  - [de422da](https://github.com/tauri-apps/tauri-mobile/commit/de422daecb6fe1cc0f45fcdd12d0119be4bd666f) Add xcode script back and skip it when building simulator target ([#108](https://github.com/tauri-apps/tauri-mobile/pull/108)) on 2023-02-22

## \[0.2.4]

- Allow to update repo with a specific branch.
  - [9d782ad](https://github.com/tauri-apps/tauri-mobile/commit/9d782add9b992fecdef60ed97d93f62ed3cdc439) fix: allow repo to update with specific branch ([#106](https://github.com/tauri-apps/tauri-mobile/pull/106)) on 2023-02-20

## \[0.2.3]

- Fixes regression when running commands and checking status code.
  - [15b9420](https://github.com/tauri-apps/tauri-mobile/commit/15b94202784c9630a2811fcb5148e8d168a09b80) fix(bossy): regression on checking status code ([#102](https://github.com/tauri-apps/tauri-mobile/pull/102)) on 2023-02-19
- Fixed gradlew execution on environments like Node-API.
  - [25f77c1](https://github.com/tauri-apps/tauri-mobile/commit/25f77c19ed0265a350fef8fce6a1e4f726c56a31) feat: use duct to run gradlew ([#103](https://github.com/tauri-apps/tauri-mobile/pull/103)) on 2023-02-19

## \[0.2.2]

- Added support for opening Android Studio installed by JetBrains Toolbox
  - [448fa99](https://github.com/tauri-apps/tauri-mobile/commit/448fa9993de3a1312ee3076a5b8ed607738932ba) feat: add support for android studio which installed by jetbrains toolbox ([#88](https://github.com/tauri-apps/tauri-mobile/pull/88)) on 2023-02-08
- Increased minimum iOS version from 9 to 13
  - [ae11564](https://github.com/tauri-apps/tauri-mobile/commit/ae115647e7c80f1b03b678c3cf76b202f9f5324f) Update minimum iOS version to 13 ([#93](https://github.com/tauri-apps/tauri-mobile/pull/93)) on 2023-02-12
- Fixed ADB and xcodebuild execution on environments like Node-API.
  - [6ce6e1f](https://github.com/tauri-apps/tauri-mobile/commit/6ce6e1f2d1f4128938ddf366c41834b78873be61) fix: command execution in tauri's Node.js CLI ([#97](https://github.com/tauri-apps/tauri-mobile/pull/97)) on 2023-02-17
  - [2f7d7a0](https://github.com/tauri-apps/tauri-mobile/commit/2f7d7a0c1136da2596d7614538411b333cddeda2) fix(apple): use duct to run xcodebuild ([#98](https://github.com/tauri-apps/tauri-mobile/pull/98)) on 2023-02-17

## \[0.2.1]

- Fix `cargo mobile update` target branch and enabled `cli` feature when update.
  - [b5791ed](https://github.com/tauri-apps/tauri-mobile/commit/b5791ed37000b92db0f5beaa50d1f6c4af52a479) fix: enable cli feature when cargo mobile update, closes [#84](https://github.com/tauri-apps/tauri-mobile/pull/84) ([#86](https://github.com/tauri-apps/tauri-mobile/pull/86)) on 2023-02-02
- Fix content assignment in ios template.
  - [81b642d](https://github.com/tauri-apps/tauri-mobile/commit/81b642de9bd0c96b124ad9bca9edfbabe78f71d4) fix(template): fix variable assignment in wry's ios template ([#82](https://github.com/tauri-apps/tauri-mobile/pull/82)) on 2023-01-30

## \[0.2.0]

- Bump minor version as `0.1.5` add a new feature which was incompatible with `0.1.4`
  - [969e8ef](https://github.com/tauri-apps/tauri-mobile/commit/969e8ef41ec6f5c51086b4971cb74318ed5fa0c3) chore: bump minor for urgent release on 2023-01-15

## \[0.1.5]

- Add support for `CARGO_TARGET_DIR` and `CARGO_BUILD_TARGET_DIR` env vars.
  - [e66a6ab](https://github.com/tauri-apps/tauri-mobile/commit/e66a6ab0e5dc3b474dad6793621c499974953915) feat: improvements for lib name and cargo target dir env vars ([#73](https://github.com/tauri-apps/tauri-mobile/pull/73)) on 2023-01-06
- Allow specifying `lib_name` in `mobile.toml` file. This useful if you set `[lib].name` in `Cargo.toml`.
  - [e66a6ab](https://github.com/tauri-apps/tauri-mobile/commit/e66a6ab0e5dc3b474dad6793621c499974953915) feat: improvements for lib name and cargo target dir env vars ([#73](https://github.com/tauri-apps/tauri-mobile/pull/73)) on 2023-01-06
- Adjust `wry` template for desktop usage also.
  - [3978774](https://github.com/tauri-apps/tauri-mobile/commit/3978774e1b5e7810f3fa6833c328e3032d744e7e) Update wry template to work on desktop as well ([#76](https://github.com/tauri-apps/tauri-mobile/pull/76)) on 2023-01-13
- Update `wry` template to use the new `wry` env vars.
  - [0113d1f](https://github.com/tauri-apps/tauri-mobile/commit/0113d1fc5fcc976a8c5c9016ae55e94fcc182ea6) feat: update wry template to use the new env vars on 2022-12-30

## \[0.1.4]

- Improve error message for missing library artifact.
  - [807861a](https://github.com/tauri-apps/tauri-mobile/commit/807861acfedf50e31086db62e56d296a62638194) feat: validate library artifact existence on 2022-12-28

## \[0.1.3]

- Allow specifying an app target dir resolver via `config::App::with_target_dir_resolver`.
  - [74c150a](https://github.com/tauri-apps/tauri-mobile/commit/74c150a7ad84db516fa39a6e9c7a4454de1d5d83) feat: allow setting a custom target dir resolver ([#68](https://github.com/tauri-apps/tauri-mobile/pull/68)) on 2022-12-28

## \[0.1.2]

- Fix `android_binding!` macro usage in the `wry` template.
  - [fd68c94](https://github.com/tauri-apps/tauri-mobile/commit/fd68c9435cdac5d591e22ff92ec2b7d36f07d8a7) fix: fix android_binding! usage in wry template on 2022-12-27

## \[0.1.1]

- Show all application logs on iOS noninteractive mode.
  - [eb071b6](https://github.com/tauri-apps/tauri-mobile/commit/eb071b65c49c4bd20abbc917fa47c75273977b4f) feat(apple): show app logs, simulator noninteractive mode ([#63](https://github.com/tauri-apps/tauri-mobile/pull/63)) on 2022-12-23
- Implement noninteractive mode on iOS simulators.
  - [eb071b6](https://github.com/tauri-apps/tauri-mobile/commit/eb071b65c49c4bd20abbc917fa47c75273977b4f) feat(apple): show app logs, simulator noninteractive mode ([#63](https://github.com/tauri-apps/tauri-mobile/pull/63)) on 2022-12-23
- Fix `cargo apple run` can't work on real device.
  - [89bbe2b](https://github.com/tauri-apps/tauri-mobile/commit/89bbe2bdd30b55d5e4af91aced779d88997cfec7) Fix `cargo apple run` can't work on real device ([#59](https://github.com/tauri-apps/tauri-mobile/pull/59)) on 2022-12-26
- Added the `openssl-vendored` Cargo feature.
  - [f76d8db](https://github.com/tauri-apps/tauri-mobile/commit/f76d8db3ca8ca472aeab8d28c0e7b41c8348de9a) feat: add `openssl-vendored` feature ([#57](https://github.com/tauri-apps/tauri-mobile/pull/57)) on 2022-12-10

## \[0.1.0]

- Initial release!
  - [4f2b4f6](https://github.com/tauri-apps/tauri-mobile/commit/4f2b4f65ddd36252ee979f88ae76855ff5c5923f) feat: prepare initial release ([#54](https://github.com/tauri-apps/tauri-mobile/pull/54)) on 2022-12-06
