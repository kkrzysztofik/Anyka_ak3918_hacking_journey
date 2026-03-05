#!/bin/bash
# Clang wrapper for ARMv5TE cross-compilation.
#
# Problem: the GCC toolchain triple (arm-unknown-linux-uclibcgnueabi) is not a
# valid LLVM target because 'uclibcgnueabi' is not a recognized environment.
# We must always use armv5te-unknown-linux-gnueabi for Clang.
#
# When rustc uses this wrapper as its linker it may pass pre-link-args that
# include --target=arm-unknown-linux-uclibcgnueabi (from the built-in Rust
# target spec). Clang uses the LAST --target on the command line, so those
# args would override our good triple. We filter them out below.
#
# Flags we always inject:
#   --target   LLVM-valid triple replaces the GCC triple
#   --sysroot  uClibc-ng headers + runtime libs
#   -B         GCC crt files (crtbeginS.o, crtendS.o)
#   -L         GCC lib dir so -lgcc resolves
#   -march/-mfloat-abi/-mtune  ARMv5TE soft-float ABI
INSTALL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../arm-anykav200-crosstool-ng" && pwd)"
GCC_LIB="${INSTALL_DIR}/lib/gcc/arm-unknown-linux-uclibcgnueabi"
GCC_VER="$(ls "${GCC_LIB}" 2>/dev/null | sort -V | tail -1)"

# Strip any --target / -target arg from caller's args so it cannot override
# the known-good triple we inject above.
filtered=()
skip_next=false
for arg in "$@"; do
    if [[ "${skip_next}" == true ]]; then skip_next=false; continue; fi
    if [[ "${arg}" == --target=* ]] || [[ "${arg}" == -target=* ]]; then continue; fi
    if [[ "${arg}" == "--target" ]] || [[ "${arg}" == "-target" ]]; then skip_next=true; continue; fi
    filtered+=("${arg}")
done

exec "${INSTALL_DIR}/bin/clang" \
    --target=armv5te-unknown-linux-gnueabi \
    --sysroot="${INSTALL_DIR}/arm-unknown-linux-uclibcgnueabi/sysroot" \
    -B "${GCC_LIB}/${GCC_VER}" \
    -L "${GCC_LIB}/${GCC_VER}" \
    -march=armv5te \
    -mfloat-abi=soft \
    -mtune=arm926ej-s \
    "${filtered[@]}"
