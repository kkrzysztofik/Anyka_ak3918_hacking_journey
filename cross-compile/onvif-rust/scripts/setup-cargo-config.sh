#!/bin/bash
# Setup .cargo/config.toml with correct paths for current environment
# Detects if running in Docker or locally and sets paths accordingly

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
CARGO_CONFIG_DIR="${PROJECT_DIR}/.cargo"
CARGO_CONFIG="${CARGO_CONFIG_DIR}/config.toml"
CARGO_CONFIG_TEMPLATE="${CARGO_CONFIG_DIR}/config.toml.template"

# Detect toolchain location
#
# Priority:
# 1) Repo-vendored toolchain (preferred)
# 2) Docker-mounted toolchain (legacy)
REPO_ROOT="$(cd "${PROJECT_DIR}/../.." && pwd)"
VENDORED_TOOLCHAIN_BASE="${REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng"
DOCKER_TOOLCHAIN_BASE="/opt/arm-anykav200-crosstool-ng"

if [[ -d "${VENDORED_TOOLCHAIN_BASE}" ]]; then
    TOOLCHAIN_BASE="${VENDORED_TOOLCHAIN_BASE}"
    ENV_TYPE="Vendored"
elif [[ -d "${DOCKER_TOOLCHAIN_BASE}" ]]; then
    TOOLCHAIN_BASE="${DOCKER_TOOLCHAIN_BASE}"
    ENV_TYPE="Docker"
else
    echo "ERROR: toolchain not found." >&2
    echo "Expected one of:" >&2
    echo "  - ${VENDORED_TOOLCHAIN_BASE}" >&2
    echo "  - ${DOCKER_TOOLCHAIN_BASE}" >&2
    exit 1
fi

# Locate the clang-wrapper for ARMv5TE linking.
# The wrapper translates the GCC triple (arm-unknown-linux-uclibcgnueabi) to a
# valid LLVM triple (armv5te-unknown-linux-gnueabi), injects sysroot/crt/libgcc
# paths, and filters any --target arg passed by rustc's pre-link-args.
# Use wrapper from installed toolchain (created during build) or generate one.
if [[ -f "${TOOLCHAIN_BASE}/bin/clang-wrapper.sh" ]]; then
    CLANG_WRAPPER="${TOOLCHAIN_BASE}/bin/clang-wrapper.sh"
else
    # Generate a self-contained wrapper inside the toolchain's bin/ directory
    # This is the final fallback - normally the wrapper is created during toolchain build
    CLANG_WRAPPER="${TOOLCHAIN_BASE}/bin/clang-wrapper.sh"
    # Docker / stripped environment: generate a self-contained wrapper inside
    # the toolchain's bin/ directory so INSTALL_DIR resolves via dirname/../
    CLANG_WRAPPER="${TOOLCHAIN_BASE}/bin/clang-wrapper.sh"
    cat > "${CLANG_WRAPPER}" << 'WRAPPER_EOF'
#!/bin/bash
INSTALL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/.."
GCC_LIB="${INSTALL_DIR}/lib/gcc/arm-unknown-linux-uclibcgnueabi"
GCC_VER="$(ls "${GCC_LIB}" 2>/dev/null | sort -V | tail -1)"
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
WRAPPER_EOF
    chmod +x "${CLANG_WRAPPER}"
fi

# Create .cargo directory if it doesn't exist
mkdir -p "${CARGO_CONFIG_DIR}"

if [[ ! -f "${CARGO_CONFIG_TEMPLATE}" ]]; then
    echo "ERROR: template not found: ${CARGO_CONFIG_TEMPLATE}" >&2
    exit 1
fi

# Substitute @TOOLCHAIN_BASE@ placeholder and write config.toml
sed "s|@TOOLCHAIN_BASE@|${TOOLCHAIN_BASE}|g" "${CARGO_CONFIG_TEMPLATE}" > "${CARGO_CONFIG}"

echo "Generated ${CARGO_CONFIG} for environment: ${ENV_TYPE}"
