---
"cargo-mobile2": patch
---

Fixed starting an iOS simulator on Xcode 27, which replaced `Simulator.app` with Device Hub. When `Simulator.app` is not available, the simulator is now booted with `xcrun simctl bootstatus` and then focused in Device Hub via its `devices://device/open?id=<udid>` deep link.
