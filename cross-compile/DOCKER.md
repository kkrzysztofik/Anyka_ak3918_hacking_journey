# Anyka Cross-Compilation Environment

This directory contains the cross-compilation environment setup for Anyka AK3918 projects.

## Quick Start with Docker (Recommended for Windows)

If you have Docker installed, this is the easiest way to get started:

1. **Build the Docker image:**
   ```bash
   cd cross-compile
   ./docker-build.sh
   ```
   Or manually:
   ```bash
   docker build -t anyka-cross-compile .
   ```

2. **Use the environment:**
   ```bash
   # Interactive shell
   docker run -it --rm -v ${PWD}:/workspace anyka-cross-compile
   
   # Build ONVIF project
   docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/onvif
   
   # Build any specific app
   docker run --rm -v ${PWD}:/workspace anyka-cross-compile /bin/bash -c "cd /workspace/onvif && make"
   ```

## Traditional Setup (Ubuntu 16.04)

For the original setup instructions, see the main README.md. This involves:
- Ubuntu 16.04 (live USB recommended)
- Running `setup.sh` to install toolchain
- Manual PATH export for each session

## Toolchain Details

- **Target:** `arm-anykav200-linux-uclibcgnueabi`
- **Compiler:** 32-bit toolchain requiring 32-bit libs
- **Install Path:** `/opt/arm-anykav200-crosstool/usr/bin`
- **Source:** [Anyka SDK from qiwen repo](https://github.com/helloworld-spec/qiwen)

## Verification

After setup, verify the toolchain works:
```bash
arm-anykav200-linux-uclibcgnueabi-gcc -v
```
