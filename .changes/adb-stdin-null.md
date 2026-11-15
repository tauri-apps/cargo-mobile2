---
"cargo-mobile2": patch
---

Spawn the Android `adb` polls (`pidof`, `getprop init.svc.bootanim`, device name lookups) with a null stdin. When the CLI runs inside Node.js (e.g. `pnpm tauri android dev`) the inherited stdin is close-on-exec, so adb started with fd 0 closed, failed to reach its server, and started a second server that aborted with `could not install *smartsocket* listener`, which repeated every couple of seconds and kept logcat from ever starting.
