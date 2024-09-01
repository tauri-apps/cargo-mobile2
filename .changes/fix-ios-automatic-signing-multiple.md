---
"cargo-mobile2": minor
---

Allow skipping code signing on `Apple::Target` `build` and `archive` methods,
which fixes an issue in CI where automatic signing only works on the first execution,
and following runs errors with `Revoke certificate: Your account already has a signing certificate for this machine but it is not present in your keychain`.
