---
name: anyka-embedded-build
description: |
  Embedded systems build and deployment specialist for Anyka ARM cross-compilation and SD card deployment.
  Triggers on: "cross-compile", "deploy", "ARM build", "armv5te", "target", "SD card", "build release", "aarch".
version: 1.0.0
---

# Anyka Embedded Build and Deployment

Cross-compile Rust binaries for ARM targets and deploy to Anyka cameras via SD card or direct testing. Support both debug builds for testing and optimized release builds for production deployment.

## Cross-Compilation Setup

The project uses `armv5te-unknown-linux-uclibceabi` as the default ARM target. Host-side operations (testing, linting) require explicit `x86_64` target specification.

### Architecture Overview

```
Host Machine (x86_64)       ARM Camera (armv5te)
├─ Rust toolchain           └─ Anyka AK3918 SoC
├─ Cross compiler           └─ uClibc runtime
└─ Testing + Linting        └─ SD Card / Flash storage
```

## Building for Different Targets

### Debug Build (Development/Testing)

```bash
# Build for ARM target
cd cross-compile/onvif-rust
cargo build --target armv5te-unknown-linux-uclibceabi

# Output location
target/armv5te-unknown-linux-uclibceabi/debug/onvif_server
```

### Release Build (Production)

```bash
# Optimized binary, smaller size, no debug symbols
cargo build --release --target armv5te-unknown-linux-uclibceabi

# Output location
target/armv5te-unknown-linux-uclibceabi/release/onvif_server
```

### Host-Side Testing and Linting

ALWAYS specify x86_64 target for host-side operations:

```bash
cd cross-compile/onvif-rust

# Run tests on host machine
cargo test --target x86_64-unknown-linux-gnu

# Unit tests only
cargo test --target x86_64-unknown-linux-gnu --lib

# Specific test
cargo test --target x86_64-unknown-linux-gnu test_name -- --nocapture

# Linting/style checks
cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings

# Format check
cargo fmt --check

# Documentation
cargo doc --target x86_64-unknown-linux-gnu --no-deps
```

## Pre-Commit Build Verification

Complete build checklist before committing:

```bash
#!/bin/bash
set -e

cd cross-compile/onvif-rust

echo "🔍 Formatting check..."
cargo fmt --check

echo "🔍 Linting..."
cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings

echo "🧪 Running tests..."
cargo test --target x86_64-unknown-linux-gnu

echo "📚 Building documentation..."
cargo doc --target x86_64-unknown-linux-gnu --no-deps

echo "🔨 Building ARM release binary..."
cargo build --release --target armv5te-unknown-linux-uclibceabi

echo "✅ All checks passed!"
```

## Build Optimization

### Release Build Settings

The `Cargo.toml` should include release optimizations:

```toml
[profile.release]
opt-level = 3           # Maximum optimization
lto = true              # Link-time optimization
codegen-units = 1       # Better optimization at cost of compile time
strip = true            # Strip debug symbols
```

### Binary Size Reduction

```bash
# Check binary size
ls -lh target/armv5te-unknown-linux-uclibceabi/release/onvif_server

# Alternative: use 'bloat' tool to find large functions
cargo install cargo-bloat
cargo bloat --release --target armv5te-unknown-linux-uclibceabi -n 20
```

## Deployment to SD Card

### SD Card Layout

```
SD Card (16GB typical)
├─ boot/                      # Boot partition
│  ├─ ak_fw                   # Bootloader
│  ├─ uImage                  # Kernel
│  └─ rootfs.cpio.gz          # Initial root filesystem
├─ anyka_hack/                # Hack layer
│  ├─ onvif_server           # ONVIF binary (our build)
│  ├─ streaming_server       # Streaming binary
│  ├─ config/
│  │  └─ config.toml         # Configuration
│  ├─ scripts/
│  │  └─ deploy.sh           # Deployment script
│  └─ rootfs/                # Custom filesystem overlay
└─ test_data/                # Test captures, logs
```

### Preparation Steps

```bash
# 1. Build ARM release binary
cd cross-compile/onvif-rust
cargo build --release --target armv5te-unknown-linux-uclibceabi

# 2. Mount SD card (on Linux)
# Find device: lsblk
SD_DEVICE=/dev/sdb

# 3. Verify it's correct (IMPORTANT - check before unmounting!)
lsblk $SD_DEVICE
mount | grep $SD_DEVICE

# 4. Unmount if currently mounted
sudo umount ${SD_DEVICE}1 ${SD_DEVICE}2 2>/dev/null || true

# 5. Flash bootloader (if needed)
sudo dd if=bootloader.bin of=$SD_DEVICE bs=512 seek=0

# 6. Create partitions (first time only)
sudo parted $SD_DEVICE --script \
    mklabel msdos \
    mkpart primary fat32 1MiB 256MiB \
    mkpart primary ext4 256MiB 100%

# 7. Format partitions
sudo mkfs.vfat ${SD_DEVICE}1
sudo mkfs.ext4 ${SD_DEVICE}2

# 8. Mount
mkdir -p /tmp/sd_boot /tmp/sd_root
sudo mount ${SD_DEVICE}1 /tmp/sd_boot
sudo mount ${SD_DEVICE}2 /tmp/sd_root

# 9. Copy boot files
sudo cp boot/uImage /tmp/sd_boot/
sudo cp boot/rootfs.cpio.gz /tmp/sd_boot/

# 10. Copy our binary and config
sudo mkdir -p /tmp/sd_root/anyka_hack
sudo cp target/armv5te-unknown-linux-uclibceabi/release/onvif_server /tmp/sd_root/anyka_hack/
sudo cp config/config.toml /tmp/sd_root/anyka_hack/

# 11. Create startup script
sudo tee /tmp/sd_root/anyka_hack/start.sh > /dev/null << 'EOF'
#!/bin/sh
cd /anyka_hack
./onvif_server --config config.toml &
EOF

sudo chmod +x /tmp/sd_root/anyka_hack/start.sh

# 12. Unmount
sudo umount /tmp/sd_boot /tmp/sd_root
sync

echo "✅ SD card ready for boot"
```

## Testing on Device

### Direct Connection Testing

```bash
# 1. Insert SD card and boot camera
# 2. Wait for boot (typically 30-60 seconds)

# 3. Identify camera IP address
# Using nmap to find it:
nmap -p 8080 192.168.1.0/24

# Or check your router's DHCP clients

# 4. SSH into camera (if enabled)
ssh root@192.168.1.100

# 5. Check if ONVIF server is running
ps aux | grep onvif

# 6. View logs
tail -f /var/log/onvif_server.log

# 7. Test SOAP endpoint
curl -X POST http://192.168.1.100:8080/onvif/device_service \
    -H "Content-Type: text/xml" \
    -d '<GetCapabilities>...</GetCapabilities>'
```

### Debugging and Troubleshooting

```bash
# Check binary dependencies
file target/armv5te-unknown-linux-uclibceabi/release/onvif_server

# List required libraries
ldd target/armv5te-unknown-linux-uclibceabi/release/onvif_server

# Check if ARM binary is correct format
readelf -h target/armv5te-unknown-linux-uclibceabi/release/onvif_server

# Capture network traffic for debugging
tcpdump -i eth0 -w capture.pcap
# Transfer to host for analysis in Wireshark
scp root@192.168.1.100:/capture.pcap .
```

## Build Caching and Performance

### Clean Rebuild

```bash
# Full clean (removes all build artifacts)
cargo clean

# Rebuild everything
cargo build --release --target armv5te-unknown-linux-uclibceabi
```

### Incremental Build

```bash
# Only rebuild changed files
cargo build --release --target armv5te-unknown-linux-uclibceabi

# Should be much faster if dependencies haven't changed
```

## Cross-Compilation Troubleshooting

### "Linker not found" Error

```bash
# Ensure cross-compilation toolchain is installed
# Check what triple you're using
rustc --version --verbose

# Install target if missing
rustup target add armv5te-unknown-linux-uclibceabi

# Verify installed targets
rustup target list | grep armv5te
```

### "Segmentation fault" on Device

Common causes:
- **Incorrect architecture**: Verify you built for armv5te, not another ARM variant
- **Missing libraries**: Check with `ldd`, copy missing libs to device
- **Stack overflow**: Increase stack size or reduce allocations
- **Memory alignment**: Verify pointer alignment for ARM

```bash
# Debug with GDB on device
gdb ./onvif_server

# Common commands
(gdb) run --config config.toml
(gdb) backtrace
(gdb) print variable_name
(gdb) quit
```

## Deployment Scripts

Create helper scripts for common tasks:

```bash
#!/bin/bash
# scripts/deploy.sh

BINARY_PATH="target/armv5te-unknown-linux-uclibceabi/release/onvif_server"
CAMERA_IP="${1:-192.168.1.100}"
CAMERA_USER="${2:-root}"
CAMERA_PATH="/anyka_hack"

# Check binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found at $BINARY_PATH"
    exit 1
fi

# Copy to device
echo "📦 Deploying $BINARY_PATH to $CAMERA_IP..."
scp "$BINARY_PATH" "$CAMERA_USER@$CAMERA_IP:$CAMERA_PATH/onvif_server"

# Make executable
ssh "$CAMERA_USER@$CAMERA_IP" chmod +x "$CAMERA_PATH/onvif_server"

# Restart service
ssh "$CAMERA_USER@$CAMERA_IP" "pkill -9 onvif_server; sleep 1; $CAMERA_PATH/start.sh"

echo "✅ Deployment complete!"
echo "📡 Camera endpoint: http://$CAMERA_IP:8080/onvif/device_service"
```

## Reference

For detailed build scripts and optimization techniques, see `scripts/deploy.sh` in this skill directory.
