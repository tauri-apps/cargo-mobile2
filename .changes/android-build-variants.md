---
"cargo-mobile2": minor
---

# Android build variants

In order to support running different build variants (such as debug and
release), we now support calling `Device::run_with_application_id_suffix`.

This relates to the `applicationIdSuffix` as defined in Android development,
described [in these docs](https://developer.android.com/build/build-variants#build-types).
