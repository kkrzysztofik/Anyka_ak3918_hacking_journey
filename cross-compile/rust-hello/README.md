# Rust Hello-World for Anyka ARM

A minimal Rust hello-world application cross-compiled for the Anyka AK3918 ARM target using Cargo.

## Target Specifications

- **Architecture**: ARMv5TEJ (CPU features: swp half fastmult edsp java)
- **Float ABI**: soft (pure software floating point - NO VFP support)
- **VFP/NEON**: None (CPU does not support VFP or NEON)
- **Toolchain**: arm-anykav200-linux-uclibcgnueabi
- **OS**: Linux (uClibc)
- **Build System**: Cargo
- **Target Triple**: `arm-unknown-linux-uclibcgnueabi` (custom target specification)

## Prerequisites

- **Rust toolchain**:
  - **Option 1 (Recommended)**: Nightly Rust (for `-Z build-std` to build std from source)

    ```bash
    rustup toolchain install nightly
    rustup default nightly
    ```

  - **Option 2**: Stable Rust (may work if pre-built std is compatible, but not guaranteed)
- **Rust target**: `armv5te-unknown-linux-gnueabi` (install with `rustup target add armv5te-unknown-linux-gnueabi`)
- Anyka ARM cross-compilation toolchain installed
  - Path: `/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/`
  - Or available in PATH as `arm-unknown-linux-uclibcgnueabi-gcc`

## Important: Building Standard Library from Source

**Similar to Go's custom runtime**, this project may need to build Rust's standard library from source to ensure:

- No VFP instructions in the runtime
- Compatibility with uclibc (not glibc)
- Proper ARMv5TEJ configuration

The pre-built standard library for `armv5te-unknown-linux-gnueabi` targets glibc, which may cause linking issues with uclibc toolchain.

## Building

### Using Cargo Directly (Recommended - Idiomatic Rust)

The primary build method is using Cargo directly:

```bash
cd cross-compile/rust-hello

# Install the target (if not already installed)
rustup target add armv5te-unknown-linux-gnueabi

# Build in release mode with std built from source (recommended for uclibc compatibility)
cargo +nightly build -Z build-std=std,core,alloc,proc_macro --release --target armv5te-unknown-linux-gnueabi

# Or try without building std (may work if pre-built std is compatible)
cargo build --release --target armv5te-unknown-linux-gnueabi

# Binary will be at: target/armv5te-unknown-linux-gnueabi/release/rust-hello
```

**Note**: Building std from source (`-Z build-std`) ensures the standard library is compiled with your uclibc toolchain and doesn't contain VFP instructions. This is similar to Go's custom runtime approach.

**Important**: The project includes uclibc stubs (`src/uclibc_stubs.c`) to provide implementations for glibc-specific functions (`getauxval`, `pthread_setname_np`) that don't exist in uclibc. These are automatically compiled and linked.

### Clean Build Artifacts

**Using Cargo:**

```bash
cargo clean
```

Or remove the entire build directory:

```bash
rm -rf build target
```

## Verification

### Verify Binary Compatibility

Run the verification script to check binary compatibility:

```bash
cd cross-compile/rust-hello
./scripts/verify_rust_binary.sh
```

This will check:

- Binary exists and is ARM architecture
- ELF flags and ABI attributes
- No VFP/NEON instructions (critical for ARMv5TEJ)
- Dynamic/static linking status

### Manual Verification

After building, verify the binary is correctly compiled for ARMv5TEJ:

```bash
# Check file type
file build/bin/rust-hello

# Check architecture
readelf -h build/bin/rust-hello | grep Machine

# Check ELF flags (should show soft-float ABI: 0x5000202)
readelf -h build/bin/rust-hello | grep Flags

# Check ABI attributes (should show Tag_ABI_FP_* attributes)
arm-anykav200-linux-uclibcgnueabi-readelf -A build/bin/rust-hello | grep Tag_ABI

# Check for VFP instructions (should find NONE)
arm-anykav200-linux-uclibcgnueabi-objdump -d build/bin/rust-hello | grep -E "vldr|vstr|vmrs|vmsr|vadd\.f|vmul\.f|vmov" || echo "✅ No VFP instructions (correct)"

# Check for NEON instructions (should find NONE)
arm-anykav200-linux-uclibcgnueabi-objdump -d build/bin/rust-hello | grep -E "vld1|vst1|vadd|neon" || echo "✅ No NEON instructions (correct)"
```

Expected output:

- File type: `ELF 32-bit LSB executable, ARM, EABI5 version 1 (SYSV), statically linked` (or dynamically linked)
- Machine: `ARM`
- Flags: Should indicate soft-float ABI
- **Critical**: No VFP or NEON instructions should be found

## Project Structure

```
cross-compile/rust-hello/
├── src/
│   ├── main.rs          # Main HTTP server application
│   ├── ffi.rs           # FFI bindings for C function
│   └── ffi.c            # C function implementation
├── .cargo/
│   └── config.toml      # Cross-compilation configuration
├── scripts/
│   └── verify_rust_binary.sh  # Binary verification script
├── Cargo.toml           # Rust project configuration
├── build.rs             # Build script for C FFI compilation

├── arm-unknown-linux-uclibcgnueabi.json  # Custom target specification
├── README.md            # This file
└── BUILD_STATUS.md      # Build status tracking
```

## Features

- **HTTP Server**: Async HTTP server on port 8080 using tokio + axum
- **XML Response**: Returns XML response matching go-hello format
- **Request/Response Logging**: Comprehensive logging of requests and responses
- **Graceful Shutdown**: Signal handling for clean shutdown (SIGINT, SIGTERM)
- **FFI Support**: Calls C function via FFI (similar to CGO in go-hello)
- **Cross-Compilation**: Configured for ARMv5TEJ with soft-float ABI

## Configuration Details

### Cargo Configuration (`.cargo/config.toml`)

The project uses Cargo's built-in cross-compilation support:

- **Target**: `arm-unknown-linux-uclibcgnueabi`
- **Linker**: `arm-anykav200-linux-uclibcgnueabi-gcc`
- **Compiler Flags**: `-march=armv5te -mfloat-abi=soft`
- **Rust Flags**: `-C target-feature=-neon,-vfp` to ensure no VFP/NEON

### FFI Configuration

The project includes FFI support to call C functions:

- `src/ffi.c`: Simple C function `hello_c()`
- `src/ffi.rs`: Rust FFI bindings
- `build.rs`: Build script that compiles C code with ARMv5TEJ flags

## Important Notes on ARMv5TEJ Compatibility

### No VFP/NEON Instructions

**Critical**: For ARMv5TEJ, you must ensure NO VFP or NEON instructions are present in the binary. If any are found, the binary will fail with "Illegal instruction" on the device.

The project is configured to:

- Use `-march=armv5te` (not ARMv7)
- Use `-mfloat-abi=soft` (pure software floating point)
- Disable VFP/NEON via Rust target features: `-C target-feature=-neon,-vfp`

### Verification

Always verify the binary before deployment:

```bash
./scripts/verify_rust_binary.sh
```

This script checks for VFP/NEON instructions and reports any found.

## Troubleshooting

### Rust Target Not Found

This project uses a custom target specification file (`arm-unknown-linux-uclibcgnueabi.json`). You don't need to install it via rustup - just reference the JSON file:

```bash
# Build using the target specification file
cargo build --release --target arm-unknown-linux-uclibcgnueabi.json
```

The `.cargo/config.toml` is configured to use this target automatically.

### Toolchain Not Found

If the cross-compiler is not found:

1. Ensure the toolchain is installed in one of the standard locations
2. Or ensure `arm-anykav200-linux-uclibcgnueabi-gcc` is in PATH
3. Or set `TOOLCHAIN_PREFIX` in the toolchain file manually

### FFI Compilation Errors

If FFI compilation fails:

1. Verify the cross-compiler is accessible: `arm-anykav200-linux-uclibcgnueabi-gcc --version`
2. Check sysroot path is correct in `.cargo/config.toml`
3. Ensure required C libraries are available in the sysroot

### VFP Instructions Found

If verification finds VFP instructions:

1. Check that `.cargo/config.toml` has `-C target-feature=-neon,-vfp` in rustflags
2. Verify `build.rs` uses correct C compiler flags: `-march=armv5te -mfloat-abi=soft`
3. Check that dependencies don't require VFP/NEON
4. Consider disabling FFI if not needed: `-DENABLE_FFI=OFF`

## Testing on Target

After building, copy the binary to the Anyka device:

```bash
# Copy to SD card or transfer via network
scp build/bin/rust-hello user@device:/tmp/

# On device, run:
/tmp/rust-hello
```

Expected output:

```
Hello, Anyka ARM World!
Target: arm-unknown-linux-uclibcgnueabi
Architecture: ARMv5TEJ (no VFP support)
Hello from C (FFI enabled)!
[INFO] Starting HTTP server on port 8080...
```

Then test the HTTP endpoint:

```bash
curl http://device-ip:8080/
```

Expected XML response:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<response>
    <message>hello-world</message>
</response>
```

## Comparison with go-hello

This Rust implementation provides equivalent functionality to the go-hello project:

- ✅ HTTP server on port 8080
- ✅ XML response format
- ✅ Request/response logging
- ✅ Graceful shutdown
- ✅ FFI support (C function calls)
- ✅ ARMv5TEJ cross-compilation
- ✅ Soft-float ABI

The main differences:

- Uses Rust's async runtime (tokio) instead of Go's goroutines
- Uses Cargo for dependency management instead of Go modules
- Uses axum web framework instead of Go's net/http
- Uses quick-xml for XML serialization instead of Go's encoding/xml
