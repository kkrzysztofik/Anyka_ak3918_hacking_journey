#!/bin/bash
# Stage 4: Bootstrap Rust from source with custom LLVM
# Installs to ${INSTALL_DIR}

set -euo pipefail

source "${SCRIPTS_DIR}/common.sh"

STAGE_NAME="rust"

stage_rust() {
    log_info "=========================================="
    log_info "Stage 4: Building Rust ${RUST_VERSION}"
    log_info "Target: ${RUST_TARGET}"
    log_info "=========================================="

    if has_checkpoint "${STAGE_NAME}"; then
        log_info "Skipping - checkpoint exists"
        return 0
    fi

    require_toolchain

    if [[ ! -f "${INSTALL_DIR}/bin/llvm-config" ]]; then
        log_error "LLVM not found at: ${INSTALL_DIR}/bin/llvm-config"
        log_error "Build LLVM first: ./build.sh --no-rust"
        exit 1
    fi

    # Clone Rust source if needed
    if [[ ! -d "${RUST_SRC_DIR}" ]]; then
        log_info "Cloning Rust source (version ${RUST_VERSION})..."
        mkdir -p "$(dirname "${RUST_SRC_DIR}")"
        git clone --depth 1 --branch "${RUST_VERSION}" \
            https://github.com/rust-lang/rust.git "${RUST_SRC_DIR}"
    fi

    # Add target specification for ARMv5TE uClibc
    if [[ "${ARCH}" == "armv5te" ]]; then
        add_rust_target_spec
    fi

    # Create Rust config.toml
    create_rust_config

    # Create clang wrapper
    create_clang_wrapper

    # Build Rust
    build_rust

    # Install Rust
    install_rust

    # Install rust-src component
    install_rust_src

    mark_checkpoint "${STAGE_NAME}"

    log_info "=========================================="
    log_info "Rust bootstrap completed!"
    log_info "rustc: ${INSTALL_DIR}/bin/rustc"
    log_info "cargo: ${INSTALL_DIR}/bin/cargo"
    log_info "=========================================="
}

add_rust_target_spec() {
    log_info "Adding ARMv5TE uClibc target specification..."

    local target_spec_dir="${RUST_SRC_DIR}/compiler/rustc_target/src/spec/targets"
    local target_module_name="armv5te_unknown_linux_uclibceabi"
    local target_spec_file="${target_spec_dir}/${target_module_name}.rs"

    if [[ -f "${target_spec_file}" ]]; then
        log_info "Target spec already exists"
        return 0
    fi

    cat > "${target_spec_file}" << 'RUST_EOF'
use crate::spec::{base, Target, TargetOptions};

pub fn target() -> Target {
    let base = base::linux_uclibc::opts();
    Target {
        llvm_target: "armv5te-unknown-linux-gnueabi".into(),
        metadata: crate::spec::TargetMetadata {
            description: None,
            tier: None,
            host_tools: None,
            std: None,
        },
        pointer_width: 32,
        data_layout: "e-m:e-p:32:32-Fi8-i64:64-v128:64:128-a:0:32-n32-S64".into(),
        arch: "arm".into(),
        options: TargetOptions {
            features: "+soft-float,+strict-align".into(),
            cpu: "arm926ej-s".into(),
            max_atomic_width: Some(32),
            mcount: "_mcount".into(),
            abi: "eabi".into(),
            linker: Some("clang".into()),
            ..base
        },
    }
}
RUST_EOF

    # Add to mod.rs if needed
    local targets_mod_file="${target_spec_dir}/mod.rs"
    if [[ -f "${targets_mod_file}" ]] && ! grep -q "${target_module_name}" "${targets_mod_file}"; then
        sed -i "/^mod armv/a mod ${target_module_name};" "${targets_mod_file}"
    fi
}

create_rust_config() {
    log_info "Creating Rust config.toml..."

    local config_file="${RUST_SRC_DIR}/config.toml"

    # Always remove stale config (e.g. left by old bootstrap_rust.sh with deleted wrapper path)
    rm -f "${config_file}"

    local system_rustc="${HOME}/.cargo/bin/rustc"
    local system_cargo="${HOME}/.cargo/bin/cargo"

    local target_cc="${CROSS_CC}"
    local target_cxx="${CROSS_CXX}"
    local target_ar="${CROSS_AR}"
    local target_ranlib="${CROSS_RANLIB}"

    cat > "${config_file}" << EOF
[llvm]
download-ci-llvm = false
assertions = false
clang = true

[build]
host = ["x86_64-unknown-linux-gnu"]
target = ["x86_64-unknown-linux-gnu", "${RUST_TARGET}"]
rustc = "${system_rustc}"
cargo = "${system_cargo}"
extended = true
tools = ["cargo", "rustfmt", "clippy", "rustdoc", "src", "rust-analyzer-proc-macro-srv", "rust-analyzer"]

[install]
prefix = "${INSTALL_DIR}"
libdir = "lib"
docdir = "share/doc/rust"
mandir = "share/man"
sysconfdir = "etc"

[target.x86_64-unknown-linux-gnu]
cc = "gcc"
cxx = "g++"
ar = "ar"
ranlib = "ranlib"

[target.${RUST_TARGET}]
linker = "${INSTALL_DIR}/bin/clang-wrapper.sh"
cc = "${target_cc}"
cxx = "${target_cxx}"
ar = "${target_ar}"
ranlib = "${target_ranlib}"

[rust]
optimize = true
debug = false
lto = "off"
codegen-units = 1
deny-warnings = false
EOF

    log_info "Rust config.toml created"
}

create_clang_wrapper() {
    log_info "Creating Clang wrapper script..."

    # Install to toolchain bin directory (canonical location for downstream consumers)
    local wrapper="${INSTALL_DIR}/bin/clang-wrapper.sh"

    # Ensure bin directory exists
    mkdir -p "$(dirname "${wrapper}")"

    cat > "${wrapper}" << 'WRAPPER_EOF'
#!/bin/bash
# Clang wrapper for ARMv5TE cross-compilation
# The wrapper lives at $INSTALL_DIR/bin/clang-wrapper.sh, so one level up
# (dirname of the script + "..") resolves to $INSTALL_DIR regardless of
# what the directory is named.
INSTALL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
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
    -fuse-ld="${INSTALL_DIR}/bin/arm-unknown-linux-uclibcgnueabi-ld.bfd" \
    -march=armv5te \
    -mfloat-abi=soft \
    -mtune=arm926ej-s \
    "${filtered[@]}"
WRAPPER_EOF
    chmod +x "${wrapper}"
    log_info "Clang wrapper created: ${wrapper}"
}

build_rust() {
    log_info "Building Rust (this will take 4-8 hours)..."

    cd "${RUST_SRC_DIR}"

    export PATH="${HOME}/.cargo/bin:${PATH}"
    unset CC CXX AR RANLIB CFLAGS CXXFLAGS LDFLAGS
    unset TARGET_CC TARGET_CXX TARGET_AR TARGET_RANLIB
    export CC="gcc" CXX="g++" AR="ar" RANLIB="ranlib"
    export LLVM_CONFIG="${INSTALL_DIR}/bin/llvm-config"

    python3 x.py build --stage 2 2>&1 | tee "${BUILD_DIR}/rust_build.log" || {
        log_error "Rust build failed. Check log: ${BUILD_DIR}/rust_build.log"
        exit 1
    }
}

install_rust() {
    log_info "Installing Rust..."

    cd "${RUST_SRC_DIR}"

    export PATH="${HOME}/.cargo/bin:${PATH}"
    export CC="gcc" CXX="g++" AR="ar" RANLIB="ranlib"
    export LLVM_CONFIG="${INSTALL_DIR}/bin/llvm-config"

    python3 x.py install --stage 2 2>&1 | tee -a "${BUILD_DIR}/rust_install.log" || {
        log_error "Rust installation failed"
        exit 1
    }
}

install_rust_src() {
    log_info "Installing rust-src component..."

    local rust_src_dest="${INSTALL_DIR}/lib/rustlib/src/rust"
    mkdir -p "${rust_src_dest}"

    if [[ -d "${RUST_SRC_DIR}/library" ]]; then
        rsync -a --exclude='backtrace/crates' \
              --exclude='stdarch/Cargo.toml' \
              --exclude='stdarch/crates/stdarch-verify' \
              --exclude='stdarch/crates/intrinsic-test' \
              "${RUST_SRC_DIR}/library/" "${rust_src_dest}/library/" 2>/dev/null || \
        cp -r "${RUST_SRC_DIR}/library" "${rust_src_dest}/"
    fi

    if [[ -d "${RUST_SRC_DIR}/src/llvm-project/libunwind" ]]; then
        mkdir -p "${rust_src_dest}/src/llvm-project"
        cp -r "${RUST_SRC_DIR}/src/llvm-project/libunwind" "${rust_src_dest}/src/llvm-project/"
    fi

    local rust_version=$("${INSTALL_DIR}/bin/rustc" --version 2>/dev/null | cut -d' ' -f2 || echo "unknown")
    echo "${rust_version}" > "${rust_src_dest}/version" 2>/dev/null || true

    local components_file="${INSTALL_DIR}/lib/rustlib/components"
    if [[ -f "${components_file}" ]] && ! grep -q "rust-src" "${components_file}"; then
        echo "rust-src" >> "${components_file}"
    fi

    log_info "rust-src installed"
}
