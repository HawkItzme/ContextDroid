# Quick start

Build or install the `contextdroid` binary, then call Android wrappers explicitly:

```text
contextdroid gradlew assembleDebug
contextdroid adb devices
contextdroid logcat --mode crash --package com.example.app
```

Use `contextdroid show <RUN_ID> --raw` to recover untouched output. See the repository
[README](../../../README.md) for profiles, integrations, limitations, and uninstall steps.
