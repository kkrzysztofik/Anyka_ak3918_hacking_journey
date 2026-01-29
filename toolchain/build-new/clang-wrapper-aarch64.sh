#!/bin/bash
# Clang wrapper for aarch64 target with sysroot
exec "/home/kmk/anyka-dev/toolchain/build-new/../aarch64-unknown-linux-gnu-toolchain/bin/clang" \
    --target=aarch64-unknown-linux-gnu \
    --sysroot="/home/kmk/anyka-dev/toolchain/build-new/../aarch64-unknown-linux-gnu-toolchain/aarch64-unknown-linux-gnu/sysroot" \
    -L"/home/kmk/anyka-dev/toolchain/build-new/../aarch64-unknown-linux-gnu-toolchain/aarch64-unknown-linux-gnu/sysroot/lib" \
    -L"/home/kmk/anyka-dev/toolchain/build-new/../aarch64-unknown-linux-gnu-toolchain/aarch64-unknown-linux-gnu/sysroot/usr/lib" \
    "$@"
