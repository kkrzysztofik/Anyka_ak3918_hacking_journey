#!/bin/bash
# Setup .cargo/config.toml with correct paths for current environment
# Detects if running in Docker or locally and sets paths accordingly

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
CARGO_CONFIG_DIR="${PROJECT_DIR}/.cargo"
CARGO_CONFIG="${CARGO_CONFIG_DIR}/config.toml"

# Detect environment
if [ -d "/opt/arm-anykav200-crosstool-ng" ]; then
    # Docker environment
    TOOLCHAIN_BASE="/opt/arm-anykav200-crosstool-ng"
else
    # Local environment
    TOOLCHAIN_BASE="/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng"
fi

# Create .cargo directory if it doesn't exist
mkdir -p "${CARGO_CONFIG_DIR}"

# Generate config.toml
cat > "${CARGO_CONFIG}" <<EOF
[build]
# Use new Rust toolchain target for ARMv5TE with uClibc
target = "armv5te-unknown-linux-uclibceabi"
# Use custom rustc for everything (now has host std library for build scripts)
rustc = "${TOOLCHAIN_BASE}/bin/rustc"

# Configuration for armv5te-unknown-linux-uclibceabi
# Using new LLVM/Clang toolchain with proper ARMv5TE support
[target.armv5te-unknown-linux-uclibceabi]
linker = "${TOOLCHAIN_BASE}/bin/clang"
rustflags = [
  "-C",
  "link-arg=--target=arm-unknown-linux-uclibcgnueabi",
  "-C",
  "link-arg=--sysroot=${TOOLCHAIN_BASE}/arm-unknown-linux-uclibcgnueabi/sysroot",
  "-C",
  "link-arg=-march=armv5te",
  "-C",
  "link-arg=-mfloat-abi=soft",
  "-C",
  "link-arg=-mtune=arm926ej-s",
  "-C",
  "link-arg=-Wl,--dynamic-linker=/mnt/anyka_hack/lib/ld-uClibc.so.1",
]

[env]
# ARM target compiler configuration
CC_armv5te_unknown_linux_uclibceabi = "${TOOLCHAIN_BASE}/bin/arm-unknown-linux-uclibcgnueabi-gcc"
CXX_armv5te_unknown_linux_uclibceabi = "${TOOLCHAIN_BASE}/bin/arm-unknown-linux-uclibcgnueabi-g++"
AR_armv5te_unknown_linux_uclibceabi = "${TOOLCHAIN_BASE}/bin/arm-unknown-linux-uclibcgnueabi-ar"

# Host build linker configuration
# Custom rustc defaults to gnu-lld-cc which requires CRT files we don't have
# Use system gcc linker for x86_64 host builds (proc-macros, build scripts)
[target.x86_64-unknown-linux-gnu]
linker = "/usr/bin/gcc"
EOF

echo "Generated ${CARGO_CONFIG} for environment: $(test -d /opt/arm-anykav200-crosstool-ng && echo 'Docker' || echo 'Local')"
