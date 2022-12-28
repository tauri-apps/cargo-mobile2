# Changelog

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
