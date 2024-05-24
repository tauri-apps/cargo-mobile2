---
"cargo-mobile2": minor
---

Use `config.identifier` as the package name in Android and bundle ID in iOS.

**BREAKING CHANGE:**
  - In `Config`, renamed field `domain` to `identifier`.
  - In `App`, renamed method `reverse_domain` to `reverse_identifier`.
