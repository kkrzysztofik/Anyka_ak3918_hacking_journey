# Toolchain Build System

Unified build system for the Anyka AK3918 cross-compilation toolchain.

## Quick Start

Build the complete toolchain with a single command:

```bash
cd toolchain/toolchain-builder
./build.sh
```

This builds everything: GCC (via crosstool-NG), LLVM/Clang, compiler-rt, Rust, and GDB.

## Usage

```bash
./build.sh [OPTIONS]

Options:
    --arch ARCH       Target architecture: armv5te (default) or aarch64
    --no-rust        Skip Rust bootstrap (faster for iteration)
    --no-gdb         Skip GDB rebuild
    --resume         Resume from last checkpoint (default)
    --clean          Clear all checkpoints and rebuild from scratch
    --dry-run        Show what would be built without executing
    -h, --help       Show help message
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

# See what would happen without running
./build.sh --dry-run
```

## Architecture

```
toolchain/toolchain-builder/
├── build.sh              # Unified entrypoint (single command)
├── README.md             # This file
├── scripts/              # Build scripts (tracked in git)
│   ├── common.sh         # Shared functions and path model
│   ├── inject_ct_config_fragments.py  # Config patcher
│   └── stages/           # Stage modules
│       ├── 01_toolchain.sh    # GCC via crosstool-NG
│       ├── 02_llvm.sh         # LLVM/Clang
│       ├── 03_compiler_rt.sh  # compiler-rt builtins (ARM)
│       ├── 04_rust.sh         # Rust bootstrap
│       └── 05_gdb.sh          # GDB rebuild
├── build/                # Generated artifacts (gitignored)
│   ├── .checkpoints/    # Build progress
│   ├── .build/          # crosstool-NG work
│   ├── rust/            # Rust source checkout
│   └── llvm-*/          # LLVM source checkout
└── crosstool-ng.config  # crosstool-NG configuration
```

## Checkpoints

The build system uses checkpoints to track progress. Completed stages are skipped on subsequent runs.

- View checkpoints: `ls toolchain/toolchain-builder/build/.checkpoints/`
- Resume: `build.sh --resume` (default)
- Clean: `build.sh --clean` (force full rebuild)

## Output

After successful build, the toolchain is installed to:

- **ARMv5TE**: `toolchain/arm-anykav200-crosstool-ng/`
- **ARM64**: `toolchain/aarch64-unknown-linux-gnu-toolchain/`

Key binaries:
- `bin/arm-unknown-linux-uclibcgnueabi-gcc`
- `bin/clang`
- `bin/rustc`
- `bin/cargo`
- `bin/armv5te-unknown-linux-uclibceabi-gdb`

## Prerequisites

```bash
sudo apt-get install -y \
    build-essential libncurses-dev gperf bison flex texinfo \
    help2man gawk libtool-bin automake autoconf wget git file \
    python3 python3-dev cmake ninja-build curl unzip xz-utils \
    perl pkg-config libgmp-dev libmpfr-dev
```

## For Developers

The build system is designed for easy iteration:

1. **Partial builds**: Use `--no-rust` or `--no-gdb` to skip time-consuming stages
2. **Resumability**: Checkpoints allow resuming from any stage
3. **Dry runs**: Test changes without waiting for full build
4. **Clean state**: `--clean` forces complete rebuild

### Adding New Stages

1. Add a new stage script in `scripts/stages/`
2. Source `common.sh` and define `stage_<name>()` function
3. Add `source` and function call in `build.sh`

## Troubleshooting

### Checkpoint Issues

If a stage fails and you want to retry:
```bash
rm toolchain/toolchain-builder/build/.checkpoints/<stage_name>
./build.sh --resume
```

### Clean Rebuild

To force a complete rebuild:
```bash
./build.sh --clean
```

### Verbose Output

For more detailed output during build, the scripts use `log_info`, `log_warn`, and `log_error` functions. Check the build logs in `build/*.log` for detailed error information.
