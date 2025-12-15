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

### Step 1: Navigate to the build directory

```bash
cd /home/kmk/anyka-dev/toolchain/build-new
```

### Step 2: Build the GCC toolchain

```bash
./build_toolchain.sh
```

This will:

- Download and build crosstool-NG 1.28.0
- Configure for ARMv5TEJ with uClibc-ng
- Build GCC 15.2, Binutils 2.45, and GDB 16.3
- Install to `../arm-anykav200-crosstool-ng/usr/`

### Step 3: Build LLVM/Clang (Required for Rust)

```bash
./build_llvm.sh
```

This builds LLVM 18.1.8 with Clang and LLD for cross-compilation support.

### Step 4: Bootstrap Rust from Source

```bash
./bootstrap_rust.sh
```

This will:

- Clone Rust source code
- Add `armv5te-unknown-linux-uclibceabi` target specification
- Configure Rust to use the custom LLVM
- Build Rust compiler and std library for the target
- Install to `../arm-anykav200-crosstool-ng/`

### Step 5: Verify the installation

```bash
./verify_rust.sh
```

After building, the toolchain will be available at:

- `../arm-anykav200-crosstool-ng/bin/rustc`
- `../arm-anykav200-crosstool-ng/bin/cargo`
- `../arm-anykav200-crosstool-ng/bin/clang`

For detailed toolchain build instructions, see the [Toolchain Build README](toolchain/build-new/README.md).

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
