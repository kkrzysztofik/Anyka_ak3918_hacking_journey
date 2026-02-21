---
description: Run full Rust quality gate (fmt, clippy, test, doc) using custom toolchain
agent: build
---

Run the full quality gate for the Rust ONVIF backend. Execute these commands sequentially and report results for each step:

```bash
cd cross-compile/onvif-rust
toolchain/arm-anykav200-crosstool-ng/bin/cargo fmt --check
toolchain/arm-anykav200-crosstool-ng/bin/cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu
toolchain/arm-anykav200-crosstool-ng/bin/cargo doc --no-deps
```

If any step fails, stop and report the failure with details on how to fix it. If all pass, confirm the quality gate is green.
