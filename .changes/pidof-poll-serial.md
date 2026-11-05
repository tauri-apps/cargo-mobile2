---
"cargo-mobile2": patch
---

Fix the Android run command never starting logcat when more than one device is connected: the app PID poll and logcat now target the selected device (`adb -s <serial>`), the poll gives up after about a minute (starting logcat without the PID filter), and logs adb's error output at debug level instead of discarding it.
