# ONVIF Coredump Analysis Prompt for WSL Ubuntu Development

## Overview
This document provides a comprehensive prompt for analyzing ONVIF daemon coredumps in the WSL Ubuntu development environment. The analysis **MUST** use the standardized `run_gdb_multiarch_analysis.sh ` script for all coredump analysis operations. Direct GDB usage is **PROHIBITED** - all analysis must go through the script to ensure consistency and proper environment setup.

## Quick Reference - Most Common Issues

| Issue | Error Message | Quick Fix |
|-------|---------------|-----------|
| Permission denied | `Permission denied` | `chmod +x debugging/run_gdb_multiarch_analysis.sh ` |
| Script not found | `No such file or directory` | Ensure script exists in `debugging/run_gdb_multiarch_analysis.sh ` |
| Binary not found | `Binary file not found` | Ensure debug binary exists in `cross-compile/onvif/out/` |
| Coredump not found | `Coredump file not found` | Check coredump location in `debugging/coredumps/` or current directory |
| Analysis failed | Script exits with error | **MANDATORY**: Fix the script before proceeding with analysis |

## Prerequisites

### Required Tools
- **WSL2 with Ubuntu** - Primary development environment
- **run_gdb_multiarch_analysis.sh  script** - **MANDATORY** standardized analysis script located in `debugging/run_gdb_multiarch_analysis.sh `
- **Native cross-compilation tools** - ARM toolchain for building debug binaries
- **Bash shell** - **MANDATORY** for all terminal operations

### Required Files
- **run_gdb_multiarch_analysis.sh ** - **MANDATORY** analysis script in `debugging/run_gdb_multiarch_analysis.sh `
- **Coredump file** - Located in `debugging/coredumps/` or current directory
- **Debug binary** - `onvifd_debug` (ARM-compiled with debug symbols) in `cross-compile/onvif/out/`
- **Libraries** - ARM libraries in `SD_card_contents/anyka_hack/onvif/lib/`

## Quick Start Analysis

```bash
# Navigate to project root
cd /home/kmk/anyka-dev

# Make script executable (if needed)
chmod +x debugging/run_gdb_multiarch_analysis.sh

# Run analysis with specific coredump and binary
./debugging/run_gdb_multiarch_analysis.sh  core.onvifd_debug.25148.55166 onvifd_debug

# Run analysis with default files (auto-detects coredump)
./debugging/run_gdb_multiarch_analysis.sh

# If analysis fails, fix the script before proceeding
# The script MUST work correctly before any coredump analysis can continue
```

## Detailed Analysis Workflow

### Step 1: Environment Setup
```bash
# Navigate to project root
cd /home/kmk/anyka-dev

# Verify analysis script exists and is executable
test -f debugging/run_gdb_multiarch_analysis.sh  && chmod +x debugging/run_gdb_multiarch_analysis.sh

# Check if debug binary exists
test -f cross-compile/onvif/out/onvifd_debug
```

### Step 2: Locate Coredump Files
```bash
# Check for coredumps in debugging directory
find debugging/coredumps -name "core.*" -type f

# Check for coredumps in current directory
find . -name "core.*" -type f -maxdepth 1

# Example output:
# debugging/coredumps/core.onvifd_debug.22788.52860
# debugging/coredumps/core.onvifd_debug.18122.16516

# List with file sizes
ls -lh debugging/coredumps/core.*
```

### Step 3: Verify Required Files
```bash
# Check for debug binary
test -f cross-compile/onvif/out/onvifd_debug && echo "Debug binary found" || echo "Debug binary missing"

# Check for libraries
ls -la SD_card_contents/anyka_hack/onvif/lib/*.so*

# Verify coredump file
COREDUMP="debugging/coredumps/core.onvifd_debug.22788.52860"
if [ -f "$COREDUMP" ]; then
    SIZE=$(du -h "$COREDUMP" | cut -f1)
    echo "Coredump found: $COREDUMP ($SIZE)"
fi

# Verify analysis script
test -f debugging/run_gdb_multiarch_analysis.sh  && echo "Analysis script found" || echo "Analysis script missing"
```

### Step 4: Run Analysis
```bash
# Navigate to project root
cd /home/kmk/anyka-dev

# Run comprehensive analysis using the standardized script
./debugging/run_gdb_multiarch_analysis.sh  core.onvifd_debug.22788.52860 onvifd_debug

# If the script fails, you MUST fix it before proceeding
# The script handles all GDB setup, library paths, and analysis automatically
# Direct GDB usage is PROHIBITED - all analysis must go through this script
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

### 1. Script-Based Analysis
```bash
# The run_gdb_multiarch_analysis.sh  script provides comprehensive analysis including:
# - Stack trace analysis (bt full)
# - Thread analysis (info threads, thread apply all bt)
# - Register analysis (info registers)
# - Memory analysis (info proc mappings, x/20x commands)
# - Shared library analysis (info sharedlibrary)
# - Source context (list, info args, info locals)

# Run the standardized analysis
./debugging/run_gdb_multiarch_analysis.sh  core.onvifd_debug.22788.52860 onvifd_debug
```

### 2. Custom Analysis Script Modification
```bash
# If you need additional analysis, modify the run_gdb_multiarch_analysis.sh  script
# Add custom GDB commands to the script's GDB batch execution
# DO NOT run GDB directly - always modify the script instead

# Example: Add custom memory analysis to the script
# Edit debugging/run_gdb_multiarch_analysis.sh  and add:
# --ex "x/50x \$sp-100" \
# --ex "x/50x \$pc-50" \
```

### 3. Native Cross-Compilation Analysis
```bash
# Build debug binary with native cross-compilation
cd /home/kmk/anyka-dev/cross-compile/onvif
make clean
make DEBUG=1

# The script automatically finds the binary in cross-compile/onvif/out/
# Run analysis using the standardized script
cd /home/kmk/anyka-dev
./debugging/run_gdb_multiarch_analysis.sh  core.onvifd_debug.22788.52860 onvifd_debug
```

### 4. Script Failure Handling
```bash
# If the analysis script fails, you MUST fix it before proceeding
# Common fixes:
# 1. Check script permissions: chmod +x debugging/run_gdb_multiarch_analysis.sh
# 2. Verify file paths in the script
# 3. Ensure all required files exist
# 4. Check script syntax: bash -n debugging/run_gdb_multiarch_analysis.sh

# NEVER bypass the script - always fix it instead
```

## Troubleshooting

### Script Issues
```bash
# Permission denied (MOST COMMON ISSUE)
chmod +x debugging/run_gdb_multiarch_analysis.sh

# Script not found
ls -la debugging/run_gdb_multiarch_analysis.sh

# Wrong working directory
# CRITICAL: Always run from project root
pwd
cd /home/kmk/anyka-dev

# Script syntax errors
bash -n debugging/run_gdb_multiarch_analysis.sh
```

### Analysis Script Issues
```bash
# "Binary file not found"
# Check if binary exists in expected location
test -f cross-compile/onvif/out/onvifd_debug && echo "Binary found" || echo "Binary missing"

# "Coredump file not found"
# Check if coredump exists in expected locations
test -f debugging/coredumps/core.onvifd_debug.25148.55166 && echo "Coredump found" || echo "Coredump missing"
test -f core.onvifd_debug.25148.55166 && echo "Coredump found in current dir" || echo "Coredump missing in current dir"

# "No GDB found"
# The script handles GDB detection automatically
# If this fails, check the script's GDB detection logic

# Script execution fails
# MANDATORY: Fix the script before proceeding with analysis
# Common fixes:
# 1. Check script permissions
# 2. Verify file paths in the script
# 3. Ensure all required files exist
# 4. Check script syntax
# 5. Review script error messages
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

### Script Execution Issues
```bash
# Script not found
find . -name "run_gdb_multiarch_analysis.sh " -type f

# Permission denied
chmod +x debugging/run_gdb_multiarch_analysis.sh

# Wrong working directory
# CRITICAL: Always run from project root
cd /home/kmk/anyka-dev

# Script fails with "Binary file not found"
# SOLUTION: Ensure debug binary exists in cross-compile/onvif/out/
ls -la cross-compile/onvif/out/onvifd_debug

# Script fails with "Coredump file not found"
# SOLUTION: Check coredump location in debugging/coredumps/ or current directory
ls -la debugging/coredumps/core.*
ls -la core.*
```

## Common Issues and Solutions

### Issue 1: Permission Denied
**Error**: `Permission denied`
**Solution**:
```bash
chmod +x debugging/run_gdb_multiarch_analysis.sh
```

### Issue 2: Script Not Found
**Error**: `No such file or directory`
**Solution**: Ensure script exists in correct location
```bash
test -f debugging/run_gdb_multiarch_analysis.sh  && echo "Script found" || echo "Script missing"
```

### Issue 3: Binary File Not Found
**Error**: `Binary file not found`
**Solution**: Ensure debug binary exists in expected location
```bash
test -f cross-compile/onvif/out/onvifd_debug && echo "Binary found" || echo "Binary missing"
```

### Issue 4: Coredump File Not Found
**Error**: `Coredump file not found`
**Solution**: Check coredump location
```bash
# Check in debugging directory
ls -la debugging/coredumps/core.*
# Check in current directory
ls -la core.*
```

### Issue 5: Analysis Script Fails
**Error**: Script exits with error code
**Solution**: **MANDATORY** - Fix the script before proceeding
```bash
# Check script syntax
bash -n debugging/run_gdb_multiarch_analysis.sh
# Check script permissions
ls -la debugging/run_gdb_multiarch_analysis.sh
# Review script error messages and fix accordingly
# NEVER bypass the script - always fix it instead
```

### Issue 6: Cross-Compilation Toolchain Missing
**Error**: `arm-linux-gnueabihf-gcc: command not found`
**Solution**: Install cross-compilation tools
```bash
sudo apt install gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf
```

## File Organization

### Coredump Locations
```
debugging/
├── coredumps/
│   ├── core.onvifd_debug.22788.52860    # Latest coredump (34MB)
│   └── core.onvifd_debug.18122.16516    # Archived coredump
├── run_gdb_multiarch_analysis.sh                   # MANDATORY analysis script
└── documentation/
    ├── Coredump_Analysis_Prompt.md      # This file
    ├── Debugging_Quick_Reference.md     # Quick commands
    └── RTSP_Session_Crash_Analysis.md   # Specific crash analysis

cross-compile/onvif/out/
└── onvifd_debug                          # Debug binary (ARM-compiled)

SD_card_contents/anyka_hack/onvif/
├── lib/                                  # ARM libraries
│   ├── ld-uClibc.so.0
│   ├── libuClibc-0.9.33.2.so
│   └── ... (other .so files)
└── debug_with_sources.sh                 # Device-side debug script
```

## Best Practices

### 1. Before Analysis
- [ ] Verify `run_gdb_multiarch_analysis.sh ` script exists and is executable
- [ ] Check coredump file size (should be > 1MB)
- [ ] Ensure debug binary exists in `cross-compile/onvif/out/`
- [ ] Verify library files are present in `SD_card_contents/anyka_hack/onvif/lib/`
- [ ] Ensure you're in the project root directory (`/home/kmk/anyka-dev/`)

### 2. During Analysis
- [ ] **MANDATORY**: Use only the `run_gdb_multiarch_analysis.sh ` script
- [ ] If script fails, fix it before proceeding (NEVER bypass)
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
test -f debugging/run_gdb_multiarch_analysis.sh  && chmod +x debugging/run_gdb_multiarch_analysis.sh
find debugging/coredumps -name "core.*" -type f

# 2. Verify required files
test -f cross-compile/onvif/out/onvifd_debug && echo "Debug binary found" || echo "Debug binary missing"
test -f debugging/coredumps/core.onvifd_debug.25148.55166 && echo "Coredump found" || echo "Coredump missing"

# 3. Run analysis using MANDATORY script
./debugging/run_gdb_multiarch_analysis.sh  core.onvifd_debug.25148.55166 onvifd_debug

# 4. If script fails, fix it before proceeding
# NEVER bypass the script - always fix it instead
# Common fixes:
# - Check script permissions: chmod +x debugging/run_gdb_multiarch_analysis.sh
# - Verify file paths in the script
# - Ensure all required files exist
# - Check script syntax: bash -n debugging/run_gdb_multiarch_analysis.sh

# 5. Review output
# Look for stack trace, register values, memory patterns
# Identify crash location and cause
# Document findings

# 6. Apply fix if needed
cd cross-compile/onvif
make clean
make DEBUG=1

# 7. Test fix
# The script automatically finds the binary in cross-compile/onvif/out/
cd /home/kmk/anyka-dev
./debugging/run_gdb_multiarch_analysis.sh  core.onvifd_debug.25148.55166 onvifd_debug
```

## Related Documentation
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
1. **MANDATORY**: Use only the `run_gdb_multiarch_analysis.sh ` script for all analysis
2. If the script fails, fix it before proceeding (NEVER bypass)
3. Check this document for troubleshooting steps
4. Review the existing coredump analysis in `debugging/coredumps/`
5. Check the RTSP crash analysis documentation
6. Verify WSL Ubuntu environment setup
7. Ensure all required files are present and accessible
8. Use native cross-compilation tools for consistent builds
9. Follow the repository's WSL development workflow guidelines
10. **Remember**: Direct GDB usage is PROHIBITED - all analysis must go through the script
