---
description: Build ARM release binary and deploy to SD card location
agent: devops
---

Build and deploy the ONVIF Rust binary for the Anyka AK3918 target device.

1. First run the quality gate:
```bash
cd cross-compile/onvif-rust
toolchain/arm-anykav200-crosstool-ng/bin/cargo fmt --check
toolchain/arm-anykav200-crosstool-ng/bin/cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu
```

2. If quality gate passes, build the ARM release:
```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo build --release
```

3. Verify the binary:
```bash
file target/armv5te-unknown-linux-uclibceabi/release/onvif-rust
ls -lh target/armv5te-unknown-linux-uclibceabi/release/onvif-rust
```

4. Copy to SD card location:
```bash
cp target/armv5te-unknown-linux-uclibceabi/release/onvif-rust SD_card_contents/anyka_hack/onvif/
```

5. Also build and deploy the WebUI:
```bash
cd cross-compile/www
npm run build
```

Report the binary size and confirm deployment locations.
