---
"cargo-mobile2": patch
---

Fixed iOS simulators being listed as connected physical devices on Xcode 27, which added them to the `xcrun devicectl list devices` output. Simulators are still listed separately via `simctl`.
