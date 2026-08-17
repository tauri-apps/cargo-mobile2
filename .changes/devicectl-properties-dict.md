---
"cargo-mobile2": patch
---

Added support for the `properties` dictionary returned by `xcrun devicectl list devices` on Xcode 27, which deprecates the `hardwareProperties`, `deviceProperties` and `connectionProperties` fields. The deprecated fields are still read when available, so older Xcode versions keep working.
