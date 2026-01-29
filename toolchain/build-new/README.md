# Modern Toolchain Build

This directory contains scripts and configuration for building modern cross-compilation toolchains using crosstool-NG. The toolchain supports multiple architectures:

- **ARMv5TE** (32-bit): For Anyka AK3918 and similar embedded devices
- **aarch64** (64-bit): For modern ARM64 systems

## Supported Architectures

### ARMv5TE (Default)

- **Architecture**: ARMv5TEJ (CPU features: swp, half, fastmult, edsp, java)
- **Float ABI**: soft (pure software floating point - NO VFP support)
- **VFP/NEON**: None (CPU does not support VFP or NEON)
- **C Library**: uClibc-ng 1.0.54
- **Kernel Headers**: Linux 3.4.35
- **Target Tuple**: `arm-unknown-linux-uclibcgnueabi`
- **Rust Target**: `armv5te-unknown-linux-uclibceabi`

### aarch64

- **Architecture**: aarch64 (64-bit ARM)
- **Float ABI**: hard (hardware floating point)
- **C Library**: glibc 2.38
- **Kernel Headers**: Linux 6.1.0
- **Target Tuple**: `aarch64-unknown-linux-gnu`
- **Rust Target**: `aarch64-unknown-linux-gnu` (builtin)

## Toolchain Components

Both architectures use the same toolchain component versions:

- crosstool-NG: 1.28.0
- GCC: 15.2
- Binutils: 2.45
- GDB: 16.3
- LLVM/Clang: 21.1.8 (optional, for Rust support)
- Rust: 1.92.0+ (optional, bootstrapped from source)

## Prerequisites

### System Requirements

- Linux or WSL environment
- At least 10GB free disk space
- 4GB+ RAM recommended
- 2-4 hours build time (depending on CPU)

### Install Build Dependencies

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
    libtool-bin \  # Note: libtool package is not enough, need libtool-bin for libtool command
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

**Note for Rust projects with OpenSSL**: If you plan to build Rust projects that use OpenSSL (like the `xiu` project with `native-tls-vendored` feature), ensure `perl` and `pkg-config` are installed. These are required for OpenSSL to be compiled from source during the Rust build process. Additionally:

- `perl` - Required by OpenSSL build scripts
- `pkg-config` - Used by openssl-sys for detection
- `make` and `gcc` - Already included in build-essential

## Building the Toolchain

### Step 1: Navigate to Build Directory

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "${REPO_ROOT}/toolchain/build-new"
```

### Step 2: Select Architecture and Build

The build script supports multiple architectures via the `ARCH` environment variable:

#### Building ARMv5TE (Default)

```bash
# Default (ARMv5TE)
./build_toolchain.sh

# Or explicitly
ARCH=armv5te ./build_toolchain.sh
```

The script will:

1. Check for required dependencies
2. Download and build crosstool-NG 1.28.0
3. Configure the toolchain for ARMv5TEJ with uClibc-ng
4. Build the complete toolchain (1-3 hours)
5. Install to `../arm-anykav200-crosstool-ng/usr/`
6. Verify the installation

#### Building aarch64

```bash
ARCH=aarch64 ./build_toolchain.sh
```

The script will:

1. Check for required dependencies
2. Download and build crosstool-NG 1.28.0 (if not already built)
3. Configure the toolchain for aarch64 with glibc
4. Build the complete toolchain (1-3 hours)
5. Install to `../aarch64-unknown-linux-gnu-toolchain/usr/`
6. Verify the installation

#### Building Both Architectures

To build both architectures in sequence:

```bash
./build_all_architectures.sh
```

This will build ARMv5TE first, then aarch64.

### Step 3: Verify the Build

After the build completes, verify the toolchain:

**For ARMv5TE:**

```bash
./verify_toolchain.sh new
```

**For aarch64:**

```bash
./verify_toolchain_aarch64.sh
```

After building, the toolchains will be available at:

- ARMv5TE: `../arm-anykav200-crosstool-ng/usr/bin/arm-unknown-linux-uclibcgnueabi-gcc`
- aarch64: `../aarch64-unknown-linux-gnu-toolchain/usr/bin/aarch64-unknown-linux-gnu-gcc`

## Configuration

The toolchain is configured via `crosstool-ng.config` file. Key settings:

- Target: `arm-unknown-linux-uclibcgnueabi`
- Architecture: ARMv5TEJ
- Float ABI: soft
- C Library: uClibc-ng 1.0.54
- Kernel Headers: Linux 3.4.35
- GCC: 15.2
- Binutils: 2.45
- GDB: 16.3

### Option 1: Use Menuconfig (Interactive)

After the initial configuration is created, you can customize it:

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "${REPO_ROOT}/toolchain/build-new"
./crosstool-ng-1.28.0/ct-ng menuconfig
```

Key settings to verify:

- **Target options** → **Architecture**: ARM
- **Target options** → **CPU**: armv5te
- **Target options** → **Float ABI**: soft
- **C-library**: uClibc-ng
- **C-library version**: 1.0.54
- **GCC version**: 15.2
- **Binutils version**: 2.45
- **Kernel version**: 3.4.35

### Option 2: Edit Config File Directly

Edit `crosstool-ng.config` and re-run the build script:

```bash
nano crosstool-ng.config
./build_toolchain.sh
```

## Troubleshooting

### Build Fails with "Missing Dependency"

Install the missing dependency and re-run:

```bash
sudo apt-get install <missing-package>
./build_toolchain.sh
```

### Build Fails During Compilation

1. Check available disk space: `df -h`
2. Check available memory: `free -h`
3. Reduce parallel jobs: Edit script and change `-j$(nproc)` to `-j2`
4. Check build logs in `.build/` directory

### Download Failures (SSL/Network Issues)

If downloads fail due to SSL or network issues:

1. **Manual download**: Download the tarball manually and place it in `.build/tarballs/`:

   ```bash
   cd toolchain/build-new
   mkdir -p .build/tarballs
   cd .build/tarballs

   # For GCC:
   wget --no-check-certificate https://gcc.gnu.org/pub/gcc/releases/gcc-15.2.0/gcc-15.2.0.tar.xz

   # For GDB:
   wget --no-check-certificate https://sourceware.org/pub/gdb/releases/gdb-16.3.tar.xz
   ```

2. **Restart build**: The build will detect the existing file and skip the download:

   ```bash
   cd toolchain/build-new
   ./build_toolchain.sh
   ```

### Configuration Errors

If configuration is invalid:

1. Delete `.config` and `crosstool-ng.config`
2. Re-run the build script to regenerate
3. Or start from a known-good sample:

   ```bash
   ./crosstool-ng-1.28.0/ct-ng arm-unknown-linux-uclibcgnueabi
   ./crosstool-ng-1.28.0/ct-ng menuconfig
   ```

### GDB Version Not Available

If GDB 16.3 is not available in crosstool-NG 1.28.0:

1. Check available versions:

   ```bash
   ./crosstool-ng-1.28.0/ct-ng menuconfig
   # Navigate to Debug facilities → GDB version
   ```

2. Select the latest available version
3. Or build GDB separately if needed

## Build Output

After successful build, the toolchain will be installed at:

```text
toolchain/arm-anykav200-crosstool-ng/usr/
├── bin/
│   ├── arm-unknown-linux-uclibcgnueabi-gcc
│   ├── arm-unknown-linux-uclibcgnueabi-g++
│   ├── arm-unknown-linux-uclibcgnueabi-ld
│   └── ...
├── arm-unknown-linux-uclibcgnueabi/
│   └── sysroot/
│       ├── lib/
│       ├── usr/
│       └── ...
└── ...
```

## Building LLVM/Clang (Optional)

LLVM/Clang is required for Rust support. The build process is split into two stages:

### Stage 1: Build LLVM/Clang Core

1. **Prerequisites**: The GCC toolchain must be built first (see above)

2. **Build LLVM/Clang**:

   ```bash
   # For ARMv5TE (default)
   ./build_llvm.sh
   
   # For aarch64
   ARCH=aarch64 ./build_llvm.sh
   ```

3. The script will:
   - Download LLVM 21.1.8 source code (latest stable)
   - Configure for native build (LLVM itself is built natively)
   - Build Clang and LLD (compiler-rt builtins are skipped in this stage)
   - Install to the appropriate toolchain directory (alongside GCC)
   - Build time: 2-4 hours

4. **Note**: Compiler-rt builtins are not built in this stage because they require cross-compilation with the target GCC, which conflicts with building LLVM natively.

### Stage 2: Build Compiler-RT Builtins (Required for Rust)

After Stage 1 completes, build the compiler-rt builtins separately:

1. **Build compiler-rt builtins**:

   ```bash
   # For ARMv5TE (default)
   ./build_compiler_rt_builtins.sh
   
   # For aarch64
   ARCH=aarch64 ./build_compiler_rt_builtins.sh
   ```

2. The script will:
   - Use the target GCC cross-compiler to build compiler-rt builtins
   - Configure for cross-compilation (target architecture)
   - Build only the builtins library (sanitizers, etc. are disabled)
   - Install builtins to the toolchain directory
   - Build time: 30-60 minutes

3. **Why two stages?**
   - LLVM/Clang must be built natively (host compiler) for performance
   - Compiler-rt builtins must be cross-compiled (target compiler) for the target architecture
   - These requirements conflict, so we build them separately

### Verify Installation

After both stages complete:

```bash
# Check LLVM tools
../arm-anykav200-crosstool-ng/bin/clang --version
../arm-anykav200-crosstool-ng/bin/llvm-config --version

# Check compiler-rt builtins (ARMv5TE)
ls -lh ../arm-anykav200-crosstool-ng/lib/clang/21.1.8/lib/linux/libclang_rt.builtins-arm.a

# Check compiler-rt builtins (aarch64)
ls -lh ../aarch64-unknown-linux-gnu-toolchain/lib/clang/21.1.8/lib/linux/libclang_rt.builtins-aarch64.a
```

After building, LLVM/Clang will be available at:

- ARMv5TE: `../arm-anykav200-crosstool-ng/bin/clang`
- aarch64: `../aarch64-unknown-linux-gnu-toolchain/bin/clang`

## Bootstrapping Rust from Source (Optional)

Rust can be bootstrapped from source using the custom LLVM toolchain. The bootstrap script supports both architectures.

1. **Prerequisites**:
   - GCC toolchain must be built for the target architecture
   - LLVM/Clang must be built (Stage 1: `./build_llvm.sh`)
   - Compiler-rt builtins must be built (Stage 2: `./build_compiler_rt_builtins.sh`)

2. **Bootstrap Rust for ARMv5TE**:

   ```bash
   ARCH=armv5te ./bootstrap_rust.sh
   ```

   The script will:
   - Clone Rust source code (stable version)
   - Add `armv5te-unknown-linux-uclibceabi` target specification
   - Configure Rust to use custom LLVM
   - Build Rust compiler and std library for the target
   - Install to `../arm-anykav200-crosstool-ng/`
   - Build time: 4-8 hours

3. **Bootstrap Rust for aarch64**:

   ```bash
   ARCH=aarch64 ./bootstrap_rust.sh
   ```

   The script will:
   - Clone Rust source code (stable version)
   - Configure Rust to use custom LLVM (aarch64 is a builtin target)
   - Build Rust compiler and std library for the target
   - Install to `../aarch64-unknown-linux-gnu-toolchain/`
   - Build time: 4-8 hours

4. **Verify installation**:

   **For ARMv5TE:**

   ```bash
   ./verify_rust.sh
   ```

   **For aarch64:**

   ```bash
   # Verification script would need to be created or use rustc directly
   ../aarch64-unknown-linux-gnu-toolchain/bin/rustc --version
   ```

5. After building, Rust will be available at:
   - ARMv5TE: `../arm-anykav200-crosstool-ng/bin/rustc`
   - aarch64: `../aarch64-unknown-linux-gnu-toolchain/bin/rustc`

6. **Using the Rust targets**:

   **For ARMv5TE**, add to your project's `.cargo/config.toml`:

   ```toml
   [build]
   target = "armv5te-unknown-linux-uclibceabi"

   [target.armv5te-unknown-linux-uclibceabi]
   linker = "/path/to/arm-anykav200-crosstool-ng/bin/clang"
   ```

   **For aarch64**, add to your project's `.cargo/config.toml`:

   ```toml
   [build]
   target = "aarch64-unknown-linux-gnu"

   [target.aarch64-unknown-linux-gnu]
   linker = "/path/to/aarch64-unknown-linux-gnu-toolchain/bin/clang"
   ```

   Then build with:

   ```bash
   # ARMv5TE
   cargo build --target armv5te-unknown-linux-uclibceabi --release

   # aarch64
   cargo build --target aarch64-unknown-linux-gnu --release
   ```

## Installing rust-src Component

The `rust-src` component contains the Rust standard library source code, which is required by rust-analyzer for IDE features like "go to definition" and hover documentation.

### Option 1: Quick Installation (Recommended)

If you already have the custom Rust toolchain built and just need to add `rust-src`:

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "${REPO_ROOT}/toolchain/build-new"
./install_rust_src.sh
```

This script will:

- Install the `rust-src` component to your existing toolchain
- Verify the installation
- Takes only a few minutes (vs 4-8 hours for full rebuild)

**Prerequisites**: The Rust source directory (`rust/`) must still exist from the original bootstrap build.

### Option 2: Full Rebuild

If you're building the toolchain from scratch or the Rust source directory was deleted:

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "${REPO_ROOT}/toolchain/build-new"
./bootstrap_rust.sh
```

The bootstrap script now automatically includes `rust-src` component installation.

### Verifying rust-src Installation

After installation, verify that rust-src is available:

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
TOOLCHAIN_DIR="${REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng"

# Check components file
grep rust-src "${TOOLCHAIN_DIR}/lib/rustlib/components"

# Check source directory exists
ls -la "${TOOLCHAIN_DIR}/lib/rustlib/src/rust/library/"
```

Expected output:

- `rust-src` appears in the components list
- Source directory contains `std/`, `core/`, `alloc/` subdirectories

### Using with rust-analyzer

After installing `rust-src`, restart VS Code or reload the window. rust-analyzer should now be able to:

- Show documentation on hover for standard library types
- Navigate to standard library source code with "Go to Definition"
- Provide accurate auto-completion for std library items

## Target Specifications

### ARMv5TE Target

The `armv5te-unknown-linux-uclibceabi` target specification is available at:

- `armv5te-unknown-linux-uclibceabi.json` (JSON format for rustup)
- Integrated into Rust source tree during bootstrap

Key characteristics:

- Architecture: ARMv5TE (arm926ej-s CPU)
- Float ABI: soft (no hardware floating point)
- C Library: uClibc-ng
- OS: Linux 3.4.35
- Linker: Clang

### aarch64 Target

The `aarch64-unknown-linux-gnu` target is a builtin Rust target:

- `aarch64-unknown-linux-gnu.json` (JSON format, provided for reference)
- Builtin target in Rust (no source modification needed)

Key characteristics:

- Architecture: aarch64 (64-bit ARM)
- Float ABI: hard (hardware floating point)
- C Library: glibc
- OS: Linux 6.1.0
- Linker: Clang

## Integration

After building, update your project's build configurations to use the new toolchain:

- Set `ANYKA_TOOLCHAIN_VERSION=new` to use the new toolchain
- Or leave unset to use the old toolchain

For Rust projects, configure Cargo as shown above.

### Next Steps

After building:

1. **Verify the toolchain**: `./verify_toolchain.sh new`
2. **Update build configurations**: See `TOOLCHAIN_INTEGRATION.md`
3. **Test with a simple project**: Build a hello-world program
4. **Migrate projects gradually**: Start with one project at a time

See the main project documentation for integration details.

## Clean Build

To start fresh:

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "${REPO_ROOT}/toolchain/build-new"
rm -rf crosstool-ng-1.28.0 .config .build
./build_toolchain.sh
```

## Notes

- The build process downloads source code automatically
- Build artifacts are stored in `.build/` directory
- Configuration is saved in `crosstool-ng.config`
- The toolchain is self-contained and can be moved/copied
