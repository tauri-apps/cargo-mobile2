---
"cargo-mobile2": patch
---

Fall back to copying the jniLibs artifact when symlink creation is denied on Windows (no Developer Mode / SeCreateSymbolicLinkPrivilege), so `tauri android build` no longer aborts on Windows machines without symlink privileges
