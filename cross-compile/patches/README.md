# ARMv5TEJ Compatibility Patches

This directory contains patches to enable Rust crates to compile for the ARMv5TEJ architecture (Anyka AK3918), which lacks native 64-bit atomic operations.

## Quick Start

```bash
# Apply all patches (downloads originals and applies diffs)
./setup.sh

# Clean and re-apply all patches
./setup.sh --clean
```

## Structure

```text
patches/
├── setup.sh              # Download and patch script
├── diffs/                # Patch files (tracked in git)
│   ├── webrtc-util-0.7.0.patch
│   ├── webrtc-ice-0.9.1.patch
│   ├── webrtc-sctp-0.8.0.patch
│   ├── rtp-0.8.0.patch
│   ├── tokio-metrics-0.2.2.patch
│   └── openssl-src-300.2.3+3.2.1.patch
├── originals/            # Downloaded crates (git-ignored)
└── *-full/               # Patched crates (git-ignored)
```

## Patched Crates

|Crate|Version|Change|
|-----|-------|-----|
|webrtc-util|0.7.0|Replace `AtomicU64` with `portable-atomic`|
|webrtc-ice|0.9.1|Replace `AtomicU64` with `portable-atomic`|
|webrtc-sctp|0.8.0|Replace `AtomicU64` with `portable-atomic`|
|rtp|0.8.0|Replace `AtomicU64` with `portable-atomic`|
|tokio-metrics|0.2.2|Replace `AtomicU64` with `portable-atomic`|
|openssl-src|300.2.3+3.2.1|Add uClibc target support|

## Why Patches?

The ARMv5TEJ architecture (ARM926EJ-S core in AK3918) lacks native 64-bit atomic operations. The standard library's `AtomicU64`/`AtomicI64` cannot be used directly. The `portable-atomic` crate provides a software fallback.

## Adding New Patches

1. Download original crate: `curl -sL "https://static.crates.io/crates/CRATE/CRATE-VERSION.crate" | tar -xz`
2. Copy to `CRATE-VERSION-full/` and make changes
3. Generate diff: `diff -ruN CRATE-VERSION CRATE-VERSION-full > diffs/CRATE-VERSION.patch`
4. Update `setup.sh` with new crate info
5. Test with `./setup.sh --clean`

## Updating Patches

To modify an existing patch:

1. Run `./setup.sh` to generate patched directories
2. Edit files in `*-full/` directory
3. Regenerate patch with diff command above
4. Test with `./setup.sh --clean`
