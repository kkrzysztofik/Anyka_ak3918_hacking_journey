# Anyka AK3918 Cross-Compilation Workspace

Cargo workspace for cross-compiling Rust applications to the Anyka AK3918 platform (ARMv5TEJ architecture with uClibc).

## Quick Start

```bash
cd cross-compile

# First time setup: apply ARMv5TEJ patches
cd patches && ./setup.sh && cd ..

# Build
cargo build

# Test
cargo test
```

## Workspace Members

| Member          | Description                         | Status       |
|-----------------|-------------------------------------|--------------|
| `onvif-rust`    | ONVIF 24.12 server implementation   | Active       |
| `streaming-lib` | Media streaming library             | Planned (T4) |

## Structure

```text
cross-compile/
├── Cargo.toml              # Workspace root with patch references
├── README.md               # This file
├── patches/                # ARMv5TEJ compatibility patches
│   ├── setup.sh            # Download and patch script
│   ├── diffs/              # Patch files (small, tracked in git)
│   └── *-full/             # Patched crates (generated, git-ignored)
└── onvif-rust/             # ONVIF 24.12 server implementation
```

## ARMv5TEJ Compatibility

The ARMv5TEJ architecture lacks native 64-bit atomic operations. Before building, run the patch setup:

```bash
cd patches
./setup.sh        # Download and apply patches
./setup.sh --clean # Clean and re-apply
```

### Patched Crates

|Crate|Version|Purpose|
|-----|-------|-------|
|`webrtc-util`|0.7.0|Replace 64-bit atomics with portable-atomic|
|`webrtc-ice`|0.9.1|Replace 64-bit atomics with portable-atomic|
|`webrtc-sctp`|0.8.0|Replace 64-bit atomics with portable-atomic|
|`rtp`|0.8.0|Replace 64-bit atomics with portable-atomic|
|`tokio-metrics`|0.2.2|Replace 64-bit atomics with portable-atomic|
|`openssl-src`|300.2.3+3.2.1|uClibc target support|

## Building

### Host Build (Development)

```bash
cargo build                    # Build all
cargo build -p onvif-rust     # Build specific member
cargo test                     # Run tests
cargo clippy -- -D warnings   # Lint
```

### Cross-Compilation (Target)

```bash
cargo build --release --target armv5te-unknown-linux-uclibceabi
```

## Cross-Compilation Setup

### Toolchain Location

```text
/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/
```

### Environment Variables

```bash
export CROSS_COMPILE=arm-anykav200-linux-uclibcgnueabi-
export CC_armv5te_unknown_linux_uclibceabi=arm-anykav200-linux-uclibcgnueabi-gcc
export AR_armv5te_unknown_linux_uclibceabi=arm-anykav200-linux-uclibcgnueabi-ar
```

## Release Profile

Optimized for embedded systems:

```toml
[profile.release]
opt-level = "z"      # Size optimization
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
strip = true         # Strip symbols
```
