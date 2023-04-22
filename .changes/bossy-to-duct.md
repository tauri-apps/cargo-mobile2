---
"tauri-mobile": "minor"
---

**Breaking** Replace `bossy` with `duct` across the crate. bossy has two ways to create commands, impure and pure. The pure version won't inherit env variables. This causes child processes won't get the env varialbes and results in issues like openssl cross compilation.
