# Changelog

## \[0.17.4]

- [`c4d420f`](https://github.com/tauri-apps/cargo-mobile2/commit/c4d420f9b9a35b52e24ad06de6c810f424ec45a3) ([#414](https://github.com/tauri-apps/cargo-mobile2/pull/414) by [@amrbashir](https://github.com/tauri-apps/cargo-mobile2/../../amrbashir)) Fix `android::emulator::avd_list` function interpreting log lines from `emulator -list-avd` as valid `Emulator`
- [`b4d615f`](https://github.com/tauri-apps/cargo-mobile2/commit/b4d615f7798afcb7c15aca02883bb931c5fa3431) ([#404](https://github.com/tauri-apps/cargo-mobile2/pull/404) by [@jmetz](https://github.com/tauri-apps/cargo-mobile2/../../jmetz)) Remove deprecated options from gradle file in the generated android template.

## \[0.17.3]

- [`1ec0ca5`](https://github.com/tauri-apps/cargo-mobile2/commit/1ec0ca542db074d8bd8351b094759011f6b712a2) ([#395](https://github.com/tauri-apps/cargo-mobile2/pull/395) by [@amrbashir](https://github.com/tauri-apps/cargo-mobile2/../../amrbashir)) Fix deserializing targets from `.cargo/config.toml` if the target's `rustflags` field is not specified
- [`e66010f`](https://github.com/tauri-apps/cargo-mobile2/commit/e66010f867f6ad4f4830fdb20a846a0ef474c1b7) ([#398](https://github.com/tauri-apps/cargo-mobile2/pull/398) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Removed name and lib name validation as they are not used as the package identifier anymore.

## \[0.17.2]

- [`cdb6ed3`](https://github.com/tauri-apps/cargo-mobile2/commit/cdb6ed362e33ffd21ebb3b6a2f1441040b7e45d1) ([#388](https://github.com/tauri-apps/cargo-mobile2/pull/388) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Only display logs from the actual iOS application unless pedantic verbosity is requested.
- [`cdb6ed3`](https://github.com/tauri-apps/cargo-mobile2/commit/cdb6ed362e33ffd21ebb3b6a2f1441040b7e45d1) ([#388](https://github.com/tauri-apps/cargo-mobile2/pull/388) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Always use verbose logging when building the app on iOS (`Target::build`) to display cargo build output.

## \[0.17.1]

- [`ce80447`](https://github.com/tauri-apps/cargo-mobile2/commit/ce804479427435cba770ffa941e27ce32b271533) ([#380](https://github.com/tauri-apps/cargo-mobile2/pull/380) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) The Rust flags for Android builds no longer need to search the .cargo folder for libraries.

## \[0.17.0]

- [`64d3e6f`](https://github.com/tauri-apps/cargo-mobile2/commit/64d3e6f04f2a6613b23caf0038812beab9554acb) ([#383](https://github.com/tauri-apps/cargo-mobile2/pull/383) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Added an `ArchiveConfig` parameter to `apple::Target::archive`.
- [`64d3e6f`](https://github.com/tauri-apps/cargo-mobile2/commit/64d3e6f04f2a6613b23caf0038812beab9554acb) ([#383](https://github.com/tauri-apps/cargo-mobile2/pull/383) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Allow skipping code signing on `Apple::Target` `build` and `archive` methods,
  which fixes an issue in CI where automatic signing only works on the first execution,
  and following runs errors with `Revoke certificate: Your account already has a signing certificate for this machine but it is not present in your keychain`.

## \[0.16.0]

- [`e289dd9`](https://github.com/tauri-apps/cargo-mobile2/commit/e289dd95a435ad069e8252519a2e1232f9376d98) ([#381](https://github.com/tauri-apps/cargo-mobile2/pull/381) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Added a `BuildConfig` argument to the `Target::build` iOS method to allow provisioning updates.
- [`e289dd9`](https://github.com/tauri-apps/cargo-mobile2/commit/e289dd95a435ad069e8252519a2e1232f9376d98) ([#381](https://github.com/tauri-apps/cargo-mobile2/pull/381) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Move `AuthCredentials` to `cargo_mobile2::apple`.

## \[0.15.1]

- [`c92d72f`](https://github.com/tauri-apps/cargo-mobile2/commit/c92d72f4a09166d54a4653d8ce9ac44296fc00c4) ([#377](https://github.com/tauri-apps/cargo-mobile2/pull/377) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Added `apple::Config::development_team` getter.

## \[0.15.0]

- [`da40535`](https://github.com/tauri-apps/cargo-mobile2/commit/da40535856cc6ca3b372e3e95b3bd59a2a391a47) ([#375](https://github.com/tauri-apps/cargo-mobile2/pull/375) by [@amrbashir](https://github.com/tauri-apps/cargo-mobile2/../../amrbashir)) The app identifier must now be provided in reverse order (e.g. `dev.tauri.app` instead of `app.tauri.dev`). Removed `App::reverse_identifier` and Added `App::identifier`.

## \[0.14.0]

- [`d0e9e58`](https://github.com/tauri-apps/cargo-mobile2/commit/d0e9e587a085d3b08a2e082dea562dbc252ad191) ([#371](https://github.com/tauri-apps/cargo-mobile2/pull/371) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Added a `ExportConfig` argument to the `Target::export` iOS method to allow provisioning updates.

## \[0.13.5]

- [`f09a6da`](https://github.com/tauri-apps/cargo-mobile2/commit/f09a6dad8c27116f1cba123038a603bdb2cd8abc) Allow hyphens on iOS identifiers and underscores on Android identifiers.

## \[0.13.4]

- [`f5548f7`](https://github.com/tauri-apps/cargo-mobile2/commit/f5548f7a522325820662d041b518eb361766358b) ([#362](https://github.com/tauri-apps/cargo-mobile2/pull/362) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Added `Config::set_export_options_plist_path` to define a custom ExportOptions.plist to use.
- [`b1e407c`](https://github.com/tauri-apps/cargo-mobile2/commit/b1e407cf21f90dfc664436703e73ec1f819d6438) ([#359](https://github.com/tauri-apps/cargo-mobile2/pull/359) by [@amrbashir](https://github.com/tauri-apps/cargo-mobile2/../../amrbashir)) Update `windows` crate to `0.58`

## \[0.13.3]

- [`b1c2313`](https://github.com/tauri-apps/cargo-mobile2/commit/b1c2313a2ab31e7e8e166b8068dce94b8b28000f) ([#353](https://github.com/tauri-apps/cargo-mobile2/pull/353) by [@mrguiman](https://github.com/tauri-apps/cargo-mobile2/../../mrguiman)) Do not include the target arch when building and archiving the iOS application.

## \[0.13.2]

- [`48c7f8e`](https://github.com/tauri-apps/cargo-mobile2/commit/48c7f8ec7b60feae5b04c45cb630a945696126f6) Added `android::Device::serial_no` getter.

## \[0.13.1]

- [`71d648f`](https://github.com/tauri-apps/cargo-mobile2/commit/71d648f16478e0fe867375ec933c4deb97406124) Update handlebars to v6.

## \[0.13.0]

- [`aad5655`](https://github.com/tauri-apps/cargo-mobile2/commit/aad5655bfeb9c14c72e30e218792a0b586709594) ([#354](https://github.com/tauri-apps/cargo-mobile2/pull/354) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Expose `apple::Device::kind`.
- [`aad5655`](https://github.com/tauri-apps/cargo-mobile2/commit/aad5655bfeb9c14c72e30e218792a0b586709594) ([#354](https://github.com/tauri-apps/cargo-mobile2/pull/354) by [@lucasfernog](https://github.com/tauri-apps/cargo-mobile2/../../lucasfernog)) Changed the `android::adb::adb` function to be generic.

## \[0.12.2]

- [`52c2905`](https://github.com/tauri-apps/cargo-mobile2/commit/52c290526debb0a26b0128cc587c542db50bc847) ([#343](https://github.com/tauri-apps/cargo-mobile2/pull/343)) Update `windows` crate to `0.57`

## \[0.12.1]

- [`7d260ba`](https://github.com/tauri-apps/cargo-mobile2/commit/7d260ba290beb39c57863eaa8a8a523ede20093b)([#328](https://github.com/tauri-apps/cargo-mobile2/pull/328)) On Android, allows using Kotlin keywords as identifiers and escape them in templates.

## \[0.12.0]

- [`adb2846`](https://github.com/tauri-apps/cargo-mobile2/commit/adb2846ab60642b3cc0a950e60c8c0f9c05f6cb5)([#297](https://github.com/tauri-apps/cargo-mobile2/pull/297)) Fix creating a new `bevy` project.
- [`29921ff`](https://github.com/tauri-apps/cargo-mobile2/commit/29921ff025ebed31546e33dc82696dc0c8fce2e0)([#330](https://github.com/tauri-apps/cargo-mobile2/pull/330)) Use `config.identifier` as the package name in Android and bundle ID in iOS.

  **BREAKING CHANGE:**

  - In `Config`, renamed field `domain` to `identifier`.
  - In `App`, renamed method `reverse_domain` to `reverse_identifier`.
- [`525d51f`](https://github.com/tauri-apps/cargo-mobile2/commit/525d51fc61e9461bd5468124554fc12d7382333f)([#305](https://github.com/tauri-apps/cargo-mobile2/pull/305)) Update `windows` crate to `0.56`
- [`2beb485`](https://github.com/tauri-apps/cargo-mobile2/commit/2beb485387e67fc14cc2b714cb457726e4cd1d77)([#298](https://github.com/tauri-apps/cargo-mobile2/pull/298)) Fix `wry` template crashing on Linux.

## \[0.11.1]

- [`cb4ed53`](https://github.com/tauri-apps/cargo-mobile2/commit/cb4ed53069f404a0eed9988b7a0dd0e29509572e)([#300](https://github.com/tauri-apps/cargo-mobile2/pull/300)) Fix `.gitignore` generated with wrong formatting.
- [`ad41fe2`](https://github.com/tauri-apps/cargo-mobile2/commit/ad41fe2328da9cb3c485f37d8081f99688463b48)([#296](https://github.com/tauri-apps/cargo-mobile2/pull/296)) Generate `.cargo/config.toml` with paths wrapped in quote.

## \[0.11.0]

- [`b370b38`](https://github.com/tauri-apps/cargo-mobile2/commit/b370b38acc8975d3f84c012354732a28edbb9e34)([#285](https://github.com/tauri-apps/cargo-mobile2/pull/285)) Fix a bug in checking for package presence when initiating an ios project
- [`0c91351`](https://github.com/tauri-apps/cargo-mobile2/commit/0c91351ef6452a8f9bad58469bca42704d8a9a1e)([#292](https://github.com/tauri-apps/cargo-mobile2/pull/292)) Remove `openssl` and use `x509-certificate` instead.
- [`1567a7a`](https://github.com/tauri-apps/cargo-mobile2/commit/1567a7a16772a2fe904e95409b74c02846de4b33)([#288](https://github.com/tauri-apps/cargo-mobile2/pull/288)) Update `windows` crate to `0.54`

## \[0.10.4]

- [`7a1066c`](https://github.com/tauri-apps/cargo-mobile2/commit/7a1066cd93d0e4cf158ccfa6a41652f2934758da)([#283](https://github.com/tauri-apps/cargo-mobile2/pull/283)) Use `adb install -r` to try replacing the android application while installing it on the device. This elimnates the need to uninstall the application from a previous run when using a real device.
- [`5a84ab2`](https://github.com/tauri-apps/cargo-mobile2/commit/5a84ab256c376e0424e2ddd6ffc44c5f0d9b5fbe)([#281](https://github.com/tauri-apps/cargo-mobile2/pull/281)) Update `wry` template to `wry@0.37`

## \[0.10.3]

- [`92eda19`](https://github.com/tauri-apps/cargo-mobile2/commit/92eda19af2a10b80470a96ade3d3dbd2a4d2af6f)([#279](https://github.com/tauri-apps/cargo-mobile2/pull/279)) Fixes log output on iOS simulators.

## \[0.10.2]

- [\`\`](https://github.com/tauri-apps/cargo-mobile2/commit/undefined) Fix adb usage for NAPI context following v0.10.1 fix.

## \[0.10.1]

- [`b12ef08`](https://github.com/tauri-apps/cargo-mobile2/commit/b12ef081c0dba630e92f924a34d4c768ef1fa522)([#277](https://github.com/tauri-apps/cargo-mobile2/pull/277)) Fix child process spawning on NAPI contexts.

## \[0.10.0]

- [`d90ccf4`](https://github.com/tauri-apps/cargo-mobile2/commit/d90ccf4c50dc180e082693b310d1f34f67d977e7)([#273](https://github.com/tauri-apps/cargo-mobile2/pull/273)) The development team configuration is now optional so you can develop on a simulator without a signing certificate.

## \[0.9.1]

- [`01d52ca`](https://github.com/tauri-apps/cargo-mobile2/commit/01d52ca2c1a582cb5a06193629d2b0bec3282ac6)([#265](https://github.com/tauri-apps/cargo-mobile2/pull/265)) Use devicectl even if iOS device is disconnected.

## \[0.9.0]

- [`cfd674e`](https://github.com/tauri-apps/cargo-mobile2/commit/cfd674e8c2f1471088bc9933be35673c9c2304d6)([#254](https://github.com/tauri-apps/cargo-mobile2/pull/254)) Fixes conflicts between Apple and Android targets. `Target::name_list` now returns `Vec<&str>`.

## \[0.8.0]

- [`cceff7e`](https://github.com/tauri-apps/cargo-mobile2/commit/cceff7e332a4b14d109b85579cc211871ef5e2d5)([#247](https://github.com/tauri-apps/cargo-mobile2/pull/247)) Fix `devicectl` listing disconnected devices.
- [`95f77b3`](https://github.com/tauri-apps/cargo-mobile2/commit/95f77b39407d7fb25925388a82268cfcd1fa1927)([#233](https://github.com/tauri-apps/cargo-mobile2/pull/233)) Update `textwrap` to 0.16.
- [`9f39389`](https://github.com/tauri-apps/cargo-mobile2/commit/9f39389cc21c552c1805c880634ca6c5df6cce7b)([#245](https://github.com/tauri-apps/cargo-mobile2/pull/245)) Update `windows` crate version to `0.51`

## \[0.7.0]

- [`739c965`](https://github.com/tauri-apps/cargo-mobile2/commit/739c965ffe7aa4bbf4162d293aea4613902bd588)([#241](https://github.com/tauri-apps/cargo-mobile2/pull/241)) Use `devicectl` on macOS 14+ to connect to iOS 17+ devices.

## \[0.6.0]

- [`5f17581`](https://github.com/tauri-apps/cargo-mobile2/commit/5f175810ee87074e67ba966461f0b0c805453971)([#219](https://github.com/tauri-apps/cargo-mobile2/pull/219)) Rename this crate to `cargo-mobile2`

## \[0.5.4]

- [`21b1386`](https://github.com/tauri-apps/tauri-mobile/commit/21b13866be1a96b01bc52d63d0b005248a014862)([#208](https://github.com/tauri-apps/tauri-mobile/pull/208)) Allow selecting "Apple Vision Pro" as an emulator.
- [`02dd3e3`](https://github.com/tauri-apps/tauri-mobile/commit/02dd3e3c37bc1d35a4e710ffd4a950815308cfd3)([#214](https://github.com/tauri-apps/tauri-mobile/pull/214)) Fix Android template generation.
- [`a82bf57`](https://github.com/tauri-apps/tauri-mobile/commit/a82bf571ec69eadd07424402def6bd8565076884)([#202](https://github.com/tauri-apps/tauri-mobile/pull/202)) Fixes `Device::run` not showing logs.
- [`a26988a`](https://github.com/tauri-apps/tauri-mobile/commit/a26988af02c13a2cce2d34b95809a8b8a4671164)([#206](https://github.com/tauri-apps/tauri-mobile/pull/206)) Add `--skip-targets-install` option for `cargo mobile new` and `cargo mobile init`

## \[0.5.3]

- [`9719aae`](https://github.com/tauri-apps/tauri-mobile/commit/9719aaeaae14560109f0ee81956f1f9083c1cc3a)([#185](https://github.com/tauri-apps/tauri-mobile/pull/185)) Fix template failing to be rendered due to missing variables on strict mode.

## \[0.5.2]

- [`4f3e4d7`](https://github.com/tauri-apps/tauri-mobile/commit/4f3e4d71af9282a2a5d054a49324909df3884a7a)([#172](https://github.com/tauri-apps/tauri-mobile/pull/172)) Fix `cargo android run` crashing because it can't detect device name using bluetooth_manager for devices without bluetooth like geneymotion.
- [`43b2a3b`](https://github.com/tauri-apps/tauri-mobile/commit/43b2a3ba3a05b9ca3d3c9d8d7eafbeb4f24bf396)([#174](https://github.com/tauri-apps/tauri-mobile/pull/174)) On Linux, fix crash after false detection of VSCode.
- [`6b8cf77`](https://github.com/tauri-apps/tauri-mobile/commit/6b8cf7758464caaa5a5cf07151cc981d04e20759)([#182](https://github.com/tauri-apps/tauri-mobile/pull/182)) Use stylized_name config for iOS product name.

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
