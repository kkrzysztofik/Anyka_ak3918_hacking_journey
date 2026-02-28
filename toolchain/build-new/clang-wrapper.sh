#!/bin/bash
# Clang wrapper for ARMv5TE target with sysroot
exec "/home/kmk/anyka-dev/toolchain/build-new/../arm-anykav200-crosstool-ng/bin/clang" \
    --target=arm-unknown-linux-uclibcgnueabi \
    --sysroot="/home/kmk/anyka-dev/toolchain/build-new/../arm-anykav200-crosstool-ng/arm-unknown-linux-uclibcgnueabi/sysroot" \
    -march=armv5te \
    -mfloat-abi=soft \
    -mtune=arm926ej-s \
    -L"/home/kmk/anyka-dev/toolchain/build-new/../arm-anykav200-crosstool-ng/arm-unknown-linux-uclibcgnueabi/sysroot/lib" \
    -L"/home/kmk/anyka-dev/toolchain/build-new/../arm-anykav200-crosstool-ng/arm-unknown-linux-uclibcgnueabi/sysroot/usr/lib" \
    "$@"
