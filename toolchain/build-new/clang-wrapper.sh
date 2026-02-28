#!/bin/bash
# Clang wrapper for ARMv5TE target with sysroot
# --gcc-toolchain tells Clang where to find GCC's crt files (crtbeginS.o, etc.)
# --sysroot  points to the uClibc-ng sysroot (headers + runtime libs)
# --target   must use an LLVM-valid environment tag (gnueabi, not uclibcgnueabi)
INSTALL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../arm-anykav200-crosstool-ng" && pwd)"
exec "${INSTALL_DIR}/bin/clang" \
    --target=armv5te-unknown-linux-gnueabi \
    --gcc-toolchain="${INSTALL_DIR}" \
    --sysroot="${INSTALL_DIR}/arm-unknown-linux-uclibcgnueabi/sysroot" \
    -march=armv5te \
    -mfloat-abi=soft \
    -mtune=arm926ej-s \
    "$@"
