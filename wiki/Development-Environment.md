# Development Environment

## Building the Custom Toolchain

The project uses a custom cross-compilation toolchain built with crosstool-NG that includes GCC, LLVM/Clang, and Rust support for the Anyka AK3918 target.

### Toolchain Build Prerequisites

- **System Requirements**:
  - Linux or WSL environment
  - At least 10GB free disk space
  - 4GB+ RAM recommended
  - 2-4 hours build time (depending on CPU)

- **Build Dependencies**:

```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    libncurses-dev \
    gperf \
    bison \
    flex \
    texinfo \
    help2man \
    gawk \
    libtool-bin \
    automake \
    autoconf \
    wget \
    git \
    file \
    python3 \
    python3-dev \
    cmake \
    ninja-build \
    curl \
    unzip \
    xz-utils \
    perl \
    pkg-config
```

## Building the Toolchain

The toolchain build system supports multiple architectures:
- **ARMv5TE** (default): For Anyka AK3918 and embedded devices
- **aarch64**: For modern ARM64 systems

### Quick Start: Single Command Build

```bash
cd /home/kmk/anyka-dev/toolchain/toolchain-builder
./build.sh
```

This builds everything: GCC, LLVM/Clang, compiler-rt, Rust, and GDB.

### Build Options

```bash
./build.sh --help

Options:
    --arch ARCH       Target architecture: armv5te (default) or aarch64
    --no-rust        Skip Rust bootstrap (faster for iteration)
    --no-gdb         Skip GDB rebuild
    --resume         Resume from last checkpoint (default)
    --clean          Clear all checkpoints and rebuild from scratch
    --dry-run        Show what would be built without executing
```

### Examples

```bash
# Full build (armv5te default)
./build.sh

# Build for ARM64
./build.sh --arch aarch64

# Build without Rust (faster)
./build.sh --no-rust

# Resume interrupted build
./build.sh --resume

# Force clean rebuild
./build.sh --clean
```

After building, the toolchains will be available at:

- ARMv5TE:
  - `../arm-anykav200-crosstool-ng/bin/rustc`
  - `../arm-anykav200-crosstool-ng/bin/cargo`
  - `../arm-anykav200-crosstool-ng/bin/clang`
- aarch64:
  - `../aarch64-unknown-linux-gnu-toolchain/bin/rustc`
  - `../aarch64-unknown-linux-gnu-toolchain/bin/cargo`
  - `../aarch64-unknown-linux-gnu-toolchain/bin/clang`

For detailed toolchain build instructions, see the [Toolchain Build README](toolchain/toolchain-builder/README.md).

## Rust Development Setup

The ONVIF Rust project uses the custom Rust toolchain for cross-compilation to the Anyka AK3918 target.

### Rust Development Prerequisites

- **Custom Toolchain**: Built using the steps above (located at `/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/`)

### Rust Development Quick Start

```bash
cd cross-compile/onvif-rust

# Use the custom toolchain's cargo
export CARGO=/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo

# Build for host (testing)
$CARGO build
$CARGO test

# Build for target (Anyka AK3918)
$CARGO build --release --target armv5te-unknown-linux-uclibceabi
```

For detailed setup instructions, see the [Developer Guide](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/cross-compile/onvif-rust/doc/DEVELOPER_GUIDE.md).

## Traditional Setup

For the original Ubuntu 16.04 setup and other legacy applications, see the [hack process documentation](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/hack_process/README.md).

## See Also

- [[ONVIF-Rust-Implementation]] - ONVIF server implementation details
- [[Development-Guide]] - Development workflow and best practices
- [[Resources]] - Additional resources and quick start guides
