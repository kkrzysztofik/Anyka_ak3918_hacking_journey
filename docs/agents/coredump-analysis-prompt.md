# ONVIF Coredump Analysis Prompt for WSL Ubuntu Development

## Overview
This document provides a comprehensive prompt for analyzing ONVIF daemon coredumps in the WSL Ubuntu development environment. The analysis leverages native cross-compilation tools and bash commands for efficient debugging and development workflow.

## Quick Reference - Most Common Issues

| Issue | Error Message | Quick Fix |
|-------|---------------|-----------|
| Permission denied | `Permission denied` | `chmod +x analyze_core.sh` |
| Library loading failed | `Could not open '/lib/ld-uClibc.so.0'` | Use native cross-compilation tools with proper library paths |
| Binary not found | `No such file or directory` | Ensure debug binary exists in `SD_card_contents/anyka_hack/onvif/` |
| Wrong directory | Various file not found errors | Run from `SD_card_contents/anyka_hack/onvif/` directory |
| GDB not found | `gdb: command not found` | Install gdb-multiarch: `sudo apt install gdb-multiarch` |

## Prerequisites

### Required Tools
- **WSL2 with Ubuntu** - Primary development environment
- **gdb-multiarch** - For ARM debugging: `sudo apt install gdb-multiarch`
- **Native cross-compilation tools** - ARM toolchain for building debug binaries
- **Bash shell** - **MANDATORY** for all terminal operations

### Required Files
- **Coredump file** - Located in `SD_card_contents/anyka_hack/onvif/` or `debugging/coredumps/`
- **Debug binary** - `onvifd_debug` (ARM-compiled with debug symbols)
- **Libraries** - ARM libraries in `SD_card_contents/anyka_hack/onvif/lib/`

## Quick Start Analysis

```bash
# Navigate to the ONVIF directory with coredumps
cd SD_card_contents/anyka_hack/onvif

# Run automatic analysis (auto-detects coredump)
./analyze_core.sh

# Or specify specific files with full paths
./analyze_core.sh core.onvifd_debug.25148.55166 onvifd_debug

# Alternative: Direct GDB analysis
gdb-multiarch --batch \
    --ex "file onvifd_debug" \
    --ex "core-file core.onvifd_debug.25148.55166" \
    --ex "bt" \
    --ex "info registers" \
    --ex "quit"
```

## Detailed Analysis Workflow

### Step 1: Environment Setup
```bash
# Navigate to project root
cd /home/kmk/anyka-dev

# Verify gdb-multiarch is installed
gdb-multiarch --version

# Check if debug binary exists
test -f SD_card_contents/anyka_hack/onvif/onvifd_debug
```

### Step 2: Locate Coredump Files
```bash
# Check for coredumps in ONVIF directory
find SD_card_contents/anyka_hack/onvif -name "core.*" -type f

# Example output:
# SD_card_contents/anyka_hack/onvif/core.onvifd_debug.22788.52860
# SD_card_contents/anyka_hack/onvif/core.onvifd_debug.18122.16516

# List with file sizes
ls -lh SD_card_contents/anyka_hack/onvif/core.*
```

### Step 3: Verify Required Files
```bash
# Check for debug binary
test -f SD_card_contents/anyka_hack/onvif/onvifd_debug && echo "Debug binary found" || echo "Debug binary missing"

# Check for libraries
ls -la SD_card_contents/anyka_hack/onvif/lib/*.so*

# Verify coredump file
COREDUMP="SD_card_contents/anyka_hack/onvif/core.onvifd_debug.22788.52860"
if [ -f "$COREDUMP" ]; then
    SIZE=$(du -h "$COREDUMP" | cut -f1)
    echo "Coredump found: $COREDUMP ($SIZE)"
fi
```

### Step 4: Run Analysis
```bash
# Navigate to ONVIF directory
cd SD_card_contents/anyka_hack/onvif

# Run comprehensive analysis
./analyze_core.sh core.onvifd_debug.22788.52860 onvifd_debug

# Or run direct GDB analysis
gdb-multiarch --batch \
    --ex "file onvifd_debug" \
    --ex "core-file core.onvifd_debug.22788.52860" \
    --ex "bt full" \
    --ex "info registers" \
    --ex "info frame" \
    --ex "list" \
    --ex "info args" \
    --ex "info locals" \
    --ex "x/20x \$pc-40" \
    --ex "x/20x \$sp-40" \
    --ex "quit"
```

## Analysis Output Interpretation

### 1. Stack Trace Analysis
Look for the call stack in the GDB output:
```
#0  0x000238b0 in rtsp_session_has_timed_out (session=0x32) at src/networking/rtsp/rtsp_session.c:35
#1  0x00023934 in rtsp_session_cleanup_timeout_sessions (server=0x91380) at src/networking/rtsp/rtsp_session.c:50
#2  0x0001fbbc in rtsp_multistream_timeout_thread (arg=0x91380) at src/networking/rtsp/rtsp_multistream.c:666
```

**Key Indicators:**
- **Function name** - Identifies the crash location
- **Parameters** - Look for invalid values (like `0x32` instead of valid pointer)
- **File and line** - Exact crash location in source code

### 2. Register Analysis
```
info registers
```
Look for:
- **PC (Program Counter)** - Crash instruction address
- **SP (Stack Pointer)** - Stack corruption indicators
- **LR (Link Register)** - Return address validity
- **General registers** - Data corruption patterns

### 3. Memory Analysis
```
x/20x $pc-40    # Memory around crash location
x/20x $sp-40    # Stack memory around SP
```

**Look for:**
- **Patterns** - Repeated values indicating corruption
- **Null pointers** - 0x00000000 or 0x00000032
- **Invalid addresses** - Values below 0x1000
- **Stack overflow** - Unusual stack patterns

### 4. Variable Inspection
```
info args       # Function arguments
info locals     # Local variables
list            # Source code around crash
```

## Advanced Analysis Techniques

### 1. Custom GDB Commands
```bash
# Create custom GDB script for specific analysis
cat > custom_analysis.gdb << 'EOF'
set solib-search-path ./lib:.:/lib:/usr/lib
file onvifd_debug
core-file core.onvifd_debug.22788.52860

# Custom analysis commands
bt full
info registers
x/50x $sp-100
x/50x $pc-50
info proc mappings
thread apply all bt
quit
EOF

# Run custom analysis
gdb-multiarch --batch --command=custom_analysis.gdb
```

### 2. Memory Leak Detection
```bash
# Check for memory leaks in coredump
gdb-multiarch --batch \
    --ex "file onvifd_debug" \
    --ex "core-file core.onvifd_debug.22788.52860" \
    --ex "info proc mappings" \
    --ex "info sharedlibrary" \
    --ex "quit"
```

### 3. Thread Analysis
```bash
# Analyze all threads in the coredump
gdb-multiarch --batch \
    --ex "file onvifd_debug" \
    --ex "core-file core.onvifd_debug.22788.52860" \
    --ex "info threads" \
    --ex "thread apply all bt" \
    --ex "quit"
```

### 4. Native Cross-Compilation Analysis
```bash
# Build debug binary with native cross-compilation
cd /home/kmk/anyka-dev/cross-compile/onvif
make clean
make DEBUG=1

# Copy debug binary to analysis directory
cp out/onvifd ../SD_card_contents/anyka_hack/onvif/onvifd_debug

# Run analysis with native tools
cd ../SD_card_contents/anyka_hack/onvif
gdb-multiarch --batch \
    --ex "file onvifd_debug" \
    --ex "core-file core.onvifd_debug.22788.52860" \
    --ex "bt full" \
    --ex "quit"
```

## Troubleshooting

### WSL/Bash Issues
```bash
# Permission denied (MOST COMMON ISSUE)
chmod +x analyze_core.sh

# Script not found
ls -la SD_card_contents/anyka_hack/onvif/analyze_core.sh

# Wrong working directory
# CRITICAL: Always run from SD_card_contents/anyka_hack/onvif/
pwd
cd /home/kmk/anyka-dev/SD_card_contents/anyka_hack/onvif

# Path issues
echo $PATH
export PATH="/usr/bin:$PATH"
```

### GDB Issues
```bash
# "gdb: command not found"
sudo apt update
sudo apt install gdb-multiarch

# "No such file or directory"
# Check if binary and coredump exist
test -f onvifd_debug && echo "Binary found" || echo "Binary missing"
test -f core.onvifd_debug.25148.55166 && echo "Coredump found" || echo "Coredump missing"

# "Could not open shared library" - "Could not open '/lib/ld-uClibc.so.0'"
# SOLUTION: Use proper library paths with native cross-compilation
export LD_LIBRARY_PATH="./lib:$LD_LIBRARY_PATH"
gdb-multiarch --batch \
    --ex "set solib-search-path ./lib:.:/lib:/usr/lib" \
    --ex "file onvifd_debug" \
    --ex "core-file core.onvifd_debug.25148.55166" \
    --ex "bt" \
    --ex "quit"

# "warning: exec file is newer than core file"
# This is normal - the binary was rebuilt after the coredump was created
# The analysis will still work correctly

# "warning: Could not load shared library symbols"
# This is normal for some libraries - analysis will still work
```

### Cross-Compilation Issues
```bash
# Toolchain not found
which arm-linux-gnueabihf-gcc
# If not found, install cross-compilation tools:
sudo apt install gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf

# Make not found
which make
sudo apt install make

# Build fails
cd /home/kmk/anyka-dev/cross-compile/onvif
make clean
make DEBUG=1
```

### Analysis Script Issues
```bash
# Script not found
find . -name "analyze_core.sh" -type f

# Permission denied
chmod +x analyze_core.sh

# Wrong working directory
# CRITICAL: Always run from SD_card_contents/anyka_hack/onvif/
cd /home/kmk/anyka-dev/SD_card_contents/anyka_hack/onvif

# Script fails with "Binary file not found"
# SOLUTION: Ensure you're in the correct directory with all required files
ls -la onvifd_debug core.* lib/
```

## Common Issues and Solutions

### Issue 1: Permission Denied
**Error**: `Permission denied`
**Solution**:
```bash
chmod +x analyze_core.sh
```

### Issue 2: Library Loading Failed
**Error**: `Could not open '/lib/ld-uClibc.so.0'`
**Solution**: Use proper library paths with native cross-compilation tools
```bash
export LD_LIBRARY_PATH="./lib:$LD_LIBRARY_PATH"
gdb-multiarch --batch \
    --ex "set solib-search-path ./lib:.:/lib:/usr/lib" \
    --ex "file onvifd_debug" \
    --ex "core-file core.onvifd_debug.25148.55166" \
    --ex "bt" \
    --ex "quit"
```

### Issue 3: GDB Not Found
**Error**: `gdb: command not found`
**Solution**: Install gdb-multiarch
```bash
sudo apt update
sudo apt install gdb-multiarch
```

### Issue 4: Wrong Working Directory
**Error**: Various file not found errors
**Solution**:
- Always run from `SD_card_contents/anyka_hack/onvif/` directory
- Ensure all required files (binary, coredump, libraries) are present

### Issue 5: Cross-Compilation Toolchain Missing
**Error**: `arm-linux-gnueabihf-gcc: command not found`
**Solution**: Install cross-compilation tools
```bash
sudo apt install gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf
```

## File Organization

### Coredump Locations
```
SD_card_contents/anyka_hack/onvif/
├── core.onvifd_debug.22788.52860    # Latest coredump (34MB)
├── onvifd_debug                     # Debug binary
├── analyze_core.sh                  # WSL analysis script
├── lib/                             # ARM libraries
│   ├── ld-uClibc.so.0
│   ├── libuClibc-0.9.33.2.so
│   └── ... (other .so files)
└── debug_with_sources.sh            # Device-side debug script

debugging/
├── coredumps/
│   └── core.onvifd_debug.18122.16516  # Archived coredump
├── analysis_scripts/
│   ├── analyze_core.sh                # WSL analysis script
│   └── run_gdb_analysis.sh            # GDB runner script
└── documentation/
    ├── Coredump_Analysis_Prompt.md    # This file
    ├── Debugging_Quick_Reference.md   # Quick commands
    └── RTSP_Session_Crash_Analysis.md # Specific crash analysis
```

## Best Practices

### 1. Before Analysis
- [ ] Verify gdb-multiarch is installed
- [ ] Check coredump file size (should be > 1MB)
- [ ] Ensure debug binary exists
- [ ] Verify library files are present
- [ ] Ensure you're in the correct directory (`SD_card_contents/anyka_hack/onvif/`)

### 2. During Analysis
- [ ] Run automatic analysis first
- [ ] Save output to file for review
- [ ] Look for patterns in multiple coredumps
- [ ] Check for known crash patterns
- [ ] Use native cross-compilation tools for consistent results

### 3. After Analysis
- [ ] Document findings
- [ ] Create fix if possible
- [ ] Test fix with new build using native make
- [ ] Archive coredump for future reference

### 4. Prevention
- [ ] Regular testing with timeout scenarios
- [ ] Memory leak detection
- [ ] Stress testing with multiple connections
- [ ] Code review for pointer validation
- [ ] Use SD-card testing for safe iteration

## Example Analysis Session

```bash
# Complete analysis session example
cd /home/kmk/anyka-dev

# 1. Check environment
gdb-multiarch --version
find SD_card_contents/anyka_hack/onvif -name "core.*" -type f

# 2. Navigate to ONVIF directory
cd SD_card_contents/anyka_hack/onvif

# 3. Run analysis using bash script
./analyze_core.sh core.onvifd_debug.25148.55166 onvifd_debug

# 4. Alternative: Run analysis using direct GDB method
gdb-multiarch --batch \
    --ex "set solib-search-path ./lib:.:/lib:/usr/lib" \
    --ex "file onvifd_debug" \
    --ex "core-file core.onvifd_debug.25148.55166" \
    --ex "bt full" \
    --ex "info registers" \
    --ex "info frame" \
    --ex "list" \
    --ex "info args" \
    --ex "info locals" \
    --ex "x/20x \$pc-40" \
    --ex "x/20x \$sp-40" \
    --ex "quit"

# 5. Review output
# Look for stack trace, register values, memory patterns
# Identify crash location and cause
# Document findings

# 6. Apply fix if needed
cd ../../cross-compile/onvif
make clean
make DEBUG=1

# 7. Test fix
cp out/onvifd ../SD_card_contents/anyka_hack/onvif/onvifd_debug
```

## Related Documentation

- [Debugging Quick Reference](Debugging_Quick_Reference.md) - Quick commands and common patterns
- [RTSP Session Crash Analysis](RTSP_Session_Crash_Analysis.md) - Detailed analysis of specific crash type
- [ONVIF Project README](../../cross-compile/onvif/README.md) - Project overview and build instructions

## Recent Analysis Example

**Coredump**: `core.onvifd_debug.25148.55166` (2.1 MB)
**Crash Location**: `ak_ai_open()` in `libplat_ai.so` at `0xb67e73c4`
**Root Cause**: Audio initialization failure during RTSP stream setup
**Signal**: SIGSEGV (Segmentation fault)
**Call Stack**: `main()` → `init_video_and_streams()` → `rtsp_multistream_server_add_stream()` → `platform_ai_open()` → `ak_ai_open()`

**Key Findings**:
- Crash occurs during ONVIF daemon startup
- Audio input initialization fails in platform library
- Zero register values suggest invalid parameters
- Issue is in vendor-provided platform library, not ONVIF code

**Recommended Fix**: Add parameter validation and error handling around `ak_ai_open()` call in `platform_ai_open()` function.

## Support

For issues with coredump analysis:
1. Check this document for troubleshooting steps
2. Review the existing coredump analysis in `debugging/coredumps/`
3. Check the RTSP crash analysis documentation
4. Verify WSL Ubuntu and gdb-multiarch environment setup
5. Ensure all required files are present and accessible
6. Use native cross-compilation tools for consistent builds
7. Follow the repository's WSL development workflow guidelines
