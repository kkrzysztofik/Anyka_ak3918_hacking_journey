---
description: Run Rust tests with the custom cargo toolchain
agent: build
---

Run the Rust test suite for the ONVIF backend using the custom toolchain:

```bash
cd cross-compile/onvif-rust
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu
```

If any tests fail, analyze the failures and suggest fixes. Show the test output including:
- Total tests run / passed / failed
- Details of any failures (test name, assertion, expected vs actual)
- Suggestions for fixing failing tests
