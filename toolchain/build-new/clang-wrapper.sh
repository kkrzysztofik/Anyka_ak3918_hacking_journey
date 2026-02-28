#!/bin/bash
# Clang wrapper for ARMv5TE target with sysroot
# --sysroot:      uClibc-ng sysroot (headers + runtime libs)
# --target:       LLVM-valid triple (gnueabi, not uclibcgnueabi)
# -B:             explicit path to GCC's crt files (crtbeginS.o, crtendS.o)
#                 required because --gcc-toolchain triple detection fails when
#                 the GCC triple (arm-unknown-linux-uclibcgnueabi) doesn't match
#                 the --target triple (armv5te-unknown-linux-gnueabi)
# -L GCC_LIB:     adds GCC lib dir to library search path so -lgcc resolves
INSTALL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../arm-anykav200-crosstool-ng" && pwd)"
GCC_LIB="${INSTALL_DIR}/lib/gcc/arm-unknown-linux-uclibcgnueabi"
GCC_VER="$(ls "${GCC_LIB}" 2>/dev/null | sort -V | tail -1)"
exec "${INSTALL_DIR}/bin/clang" \
    --target=armv5te-unknown-linux-gnueabi \
    --sysroot="${INSTALL_DIR}/arm-unknown-linux-uclibcgnueabi/sysroot" \
    -B "${GCC_LIB}/${GCC_VER}" \
    -L "${GCC_LIB}/${GCC_VER}" \
    -march=armv5te \
    -mfloat-abi=soft \
    -mtune=arm926ej-s \
    "$@"
