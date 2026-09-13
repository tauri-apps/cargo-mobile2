---
"cargo-mobile2": patch
---

Fall back to copying the jniLibs artifact when symlink creation is denied on Windows (no Developer Mode / SeCreateSymbolicLinkPrivilege), so `tauri android build` no longer aborts on Windows machines without symlink privileges

Also rewrites the reduced Chrome//Edg/ version token with the real Client Hints build so the reported version is not a frozen x.0.0.0 stub, and follows symlinks/handles malformed version tokens during the jniLibs directory copy so the fallback is robust on real-world artifact trees.

