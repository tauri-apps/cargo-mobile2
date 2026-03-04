---
"cargo-mobile2": patch
---

Added `Device::run_with_application_id_suffix` to support running different build variants. This relates to the `applicationIdSuffix` as defined in Android development,
described [in these docs](https://developer.android.com/build/build-variants#build-types).
