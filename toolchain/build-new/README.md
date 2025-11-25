# Modern Toolchain Build for Anyka AK3918

This directory contains scripts and configuration for building a modern cross-compilation toolchain for Anyka AK3918 using crosstool-NG.

## Target Specifications

- **Architecture**: ARMv5TEJ (CPU features: swp, half, fastmult, edsp, java)
- **Float ABI**: soft (pure software floating point - NO VFP support)
- **VFP/NEON**: None (CPU does not support VFP or NEON)
- **C Library**: uClibc-ng 1.0.54
- **Kernel Headers**: Linux 3.4.35
- **Toolchain Components**:
  - crosstool-NG: 1.28.0
  - GCC: 15.2
  - uClibc-ng: 1.0.54
  - Binutils: 2.45
  - GDB: 16.3
  - LLVM/Clang: 18.1.8 (optional, for Rust support)
  - Rust: 1.80.0+ (optional, bootstrapped from source)

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
cd /home/kmk/anyka-dev/toolchain/build-new
```

### Step 2: Run the Build Script

```bash
./build_toolchain.sh
```

The script will:

1. Check for required dependencies
2. Download and build crosstool-NG 1.28.0
3. Configure the toolchain for ARMv5TEJ with uClibc-ng
4. Build the complete toolchain (1-3 hours)
5. Install to `../arm-anykav200-crosstool-ng/usr/`
6. Verify the installation

### Step 3: Verify the Build

After the build completes, verify the toolchain:

```bash
./verify_toolchain.sh new
```

After building, the toolchain will be available at:

- `../arm-anykav200-crosstool-ng/usr/bin/arm-unknown-linux-uclibcgnueabi-gcc`

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
cd /home/kmk/anyka-dev/toolchain/build-new
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

LLVM/Clang is required for Rust support. To build LLVM/Clang:

1. **Prerequisites**: The GCC toolchain must be built first (see above)

2. **Build LLVM/Clang**:

   ```bash
   ./build_llvm.sh
   ```

3. The script will:
   - Download LLVM 18.1.8 source code
   - Configure for ARMv5TE cross-compilation
   - Use the GCC toolchain as sysroot
   - Build Clang, LLD, and compiler-rt
   - Install to `../arm-anykav200-crosstool-ng/` (alongside GCC)
   - Build time: 2-4 hours

4. **Verify installation**:

   ```bash
   ./verify_llvm.sh
   ```

5. After building, LLVM/Clang will be available at:
   - `../arm-anykav200-crosstool-ng/bin/clang`
   - `../arm-anykav200-crosstool-ng/bin/llvm-config`

## Bootstrapping Rust from Source (Optional)

Rust can be bootstrapped from source using the custom LLVM toolchain. This enables the `armv5te-unknown-linux-uclibceabi` target.

1. **Prerequisites**:
   - GCC toolchain must be built
   - LLVM/Clang must be built (see above)

2. **Bootstrap Rust**:

   ```bash
   ./bootstrap_rust.sh
   ```

3. The script will:
   - Clone Rust source code (stable version)
   - Add `armv5te-unknown-linux-uclibceabi` target specification
   - Configure Rust to use custom LLVM
   - Build Rust compiler and std library for the target
   - Install to `../arm-anykav200-crosstool-ng/`
   - Build time: 4-8 hours

4. **Verify installation**:

   ```bash
   ./verify_rust.sh
   ```

5. After building, Rust will be available at:
   - `../arm-anykav200-crosstool-ng/bin/rustc`
   - `../arm-anykav200-crosstool-ng/bin/cargo`

6. **Using the Rust target**:

   Add to your project's `.cargo/config.toml`:

   ```toml
   [build]
   target = "armv5te-unknown-linux-uclibceabi"

   [target.armv5te-unknown-linux-uclibceabi]
   linker = "/path/to/arm-anykav200-crosstool-ng/bin/clang"
   ```

   Then build with:

   ```bash
   cargo build --target armv5te-unknown-linux-uclibceabi --release
   ```

## Target Specification

The `armv5te-unknown-linux-uclibceabi` target specification is available at:

- `armv5te-unknown-linux-uclibceabi.json` (JSON format for rustup)
- Integrated into Rust source tree during bootstrap

Key characteristics:

- Architecture: ARMv5TE (arm926ej-s CPU)
- Float ABI: soft (no hardware floating point)
- C Library: uClibc-ng
- OS: Linux 3.4.35
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
cd /home/kmk/anyka-dev/toolchain/build-new
rm -rf crosstool-ng-1.28.0 .config .build
./build_toolchain.sh
```

## Notes

- The build process downloads source code automatically
- Build artifacts are stored in `.build/` directory
- Configuration is saved in `crosstool-ng.config`
- The toolchain is self-contained and can be moved/copied
