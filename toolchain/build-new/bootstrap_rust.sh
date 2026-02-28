#!/bin/bash
# Bootstrap Rust from source with custom LLVM
# Target: ARMv5TE (armv5te-unknown-linux-uclibceabi) for Anyka AK3918 cameras

set -e  # Exit on error

# Script directory — must be set before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

# Rust target triple (distinct from the GCC TARGET_TUPLE in common.sh)
TARGET_NAME="armv5te-unknown-linux-uclibceabi"
TARGET_SPEC="${BUILD_DIR}/${TARGET_NAME}.json"
CLANG_WRAPPER="clang-wrapper.sh"

# Check dependencies
check_dependencies() {
    log_info "Checking build dependencies..."

    # Clear any cross-compilation environment variables that might interfere
    # These will be set properly in config.toml and build functions
    unset CC CXX AR RANLIB CFLAGS CXXFLAGS LDFLAGS
    unset TARGET_CC TARGET_CXX TARGET_AR TARGET_RANLIB
    # Ensure native compiler is available for host builds
    export CC="gcc"
    export CXX="g++"
    export AR="ar"
    export RANLIB="ranlib"

    log_info "Cleared cross-compilation environment variables"
    log_info "Using native CC: ${CC}, CXX: ${CXX}"

    check_deps python3 git curl cmake ninja perl pkg-config make gcc

    # Check for LLVM
    local llvm_config="${INSTALL_DIR}/bin/llvm-config"
    if [[ ! -f "${llvm_config}" ]]; then
        log_error "LLVM not found at: ${llvm_config}"
        log_error ""
        log_error "LLVM/Clang is required for Rust bootstrap."
        log_error "Please build LLVM first using build_llvm.sh"
        log_error ""
        log_error "Run: ./build_llvm.sh"
        exit 1
    fi

    # Check for target spec file (required for armv5te-unknown-linux-uclibceabi)
    if [[ ! -f "${TARGET_SPEC}" ]]; then
        log_error "Target specification not found: ${TARGET_SPEC}"
        exit 1
    fi

    log_info "All dependencies satisfied"
    return 0
}

# Clone Rust source
clone_rust() {
    log_info "Cloning Rust source (version ${RUST_VERSION})..."

    if [[ -d "${RUST_SRC_DIR}" ]]; then
        log_warn "Rust source directory exists, skipping clone"
        log_info "To re-clone, remove: ${RUST_SRC_DIR}"
        return 0
    fi

    cd "${BUILD_DIR}"

    log_info "Cloning Rust repository (version ${RUST_VERSION})..."
    # Clone the specific version tag - do NOT fallback to stable
    # Rust tags use plain semantic versions (e.g., 1.92.0) without a v prefix
    git clone --depth 1 --branch "${RUST_VERSION}" \
        https://github.com/rust-lang/rust.git "${RUST_SRC_DIR}" || {
        log_error "Failed to clone Rust source for version ${RUST_VERSION}"
        log_error "Available versions can be checked at: https://github.com/rust-lang/rust/tags"
        exit 1
    }

    log_info "Rust source cloned. Using system Rust ${RUST_VERSION} for bootstrap."

    log_info "Rust source cloned successfully"
}

# Add target specification to Rust source
add_target_spec() {
    log_info "Adding target specification to Rust source..."

    # Modern Rust uses compiler/rustc_target/src/spec/targets/
    local target_spec_dir="${RUST_SRC_DIR}/compiler/rustc_target/src/spec/targets"
    # Convert target name to Rust module name (armv5te-unknown-linux-uclibceabi -> armv5te_unknown_linux_uclibceabi)
    local target_module_name=$(echo "${TARGET_NAME}" | tr '-' '_')
    local target_spec_file="${target_spec_dir}/${target_module_name}.rs"

    # Create directory if it doesn't exist
    if [[ ! -d "${target_spec_dir}" ]]; then
        log_error "Target spec directory not found: ${target_spec_dir}"
        log_error "Rust source structure may have changed. Please check the Rust version."
        exit 1
    fi

    if [[ -f "${target_spec_file}" ]]; then
        log_warn "Target spec already exists, skipping"
        return 0
    fi

    # Convert JSON to Rust code (modern Rust structure)
    log_info "Converting JSON target spec to Rust code..."

    cat > "${target_spec_file}" << 'RUST_EOF'
use crate::spec::{base, Target, TargetOptions};

// This target is for uclibc Linux on ARMv5TE without NEON,
// thumb-mode or hardfloat.

pub fn target() -> Target {
    let base = base::linux_uclibc::opts();
    Target {
        llvm_target: "arm-unknown-linux-uclibcgnueabi".into(),
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

    log_info "Target spec Rust file created: ${target_spec_file}"

    # Add module to mod.rs in the targets directory
    local targets_mod_file="${target_spec_dir}/mod.rs"
    if [ -f "${targets_mod_file}" ] && ! grep -q "${target_module_name}" "${targets_mod_file}"; then
        log_info "Adding target module to targets/mod.rs..."
        # Find a good place to insert (alphabetically)
        if grep -q "^mod armv" "${targets_mod_file}"; then
            # Insert after last armv* target
            sed -i "/^mod armv/a mod ${target_module_name};" "${targets_mod_file}"
        else
            # Add at end of mod declarations
            sed -i '/^mod /a mod '"${target_module_name}"';' "${targets_mod_file}" || \
            echo "mod ${target_module_name};" >> "${targets_mod_file}"
        fi
        log_info "Added module declaration to targets/mod.rs"
    fi

    # Register target in main mod.rs
    local main_mod_file="${RUST_SRC_DIR}/compiler/rustc_target/src/spec/mod.rs"
    if [ -f "${main_mod_file}" ] && ! grep -q "${TARGET_NAME}" "${main_mod_file}"; then
        log_info "Registering target in main mod.rs..."
        # Find the target registration section and add our target
        if grep -q "armv7-unknown-linux-uclibceabi" "${main_mod_file}"; then
            # Insert after armv7-unknown-linux-uclibceabi
            sed -i "/armv7-unknown-linux-uclibceabi/a\\    (\"${TARGET_NAME}\", ${target_module_name})," "${main_mod_file}"
        else
            log_warn "Could not find armv7-unknown-linux-uclibceabi in mod.rs to use as reference"
            log_warn "You may need to manually add: (\"${TARGET_NAME}\", ${target_module_name}),"
        fi
        log_info "Registered target in main mod.rs"
    fi
}

# Create Rust config.toml
create_rust_config() {
    log_info "Creating Rust config.toml for ${ARCH}..."

    local config_file="${RUST_SRC_DIR}/config.toml"
    local llvm_config="${INSTALL_DIR}/bin/llvm-config"
    local sysroot="${SYSROOT}"

    # Find system Rust toolchain for bootstrap
    local system_rustc=$(which rustc 2>/dev/null || echo "rustc")
    local system_cargo=$(which cargo 2>/dev/null || echo "cargo")

    if [[ "${system_rustc}" = "rustc" ]] || [[ "${system_cargo}" = "cargo" ]]; then
        log_warn "System rustc/cargo not found in PATH"
        log_warn "Rust bootstrap requires an existing Rust toolchain"
        log_warn "Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        log_warn "Or ensure rustc and cargo are in your PATH"
    fi

    # ARMv5TE compiler paths
    local target_cc target_cxx target_ar target_ranlib
    target_cc="${INSTALL_DIR}/bin/arm-unknown-linux-uclibcgnueabi-gcc"
    target_cxx="${INSTALL_DIR}/bin/arm-unknown-linux-uclibcgnueabi-g++"
    target_ar="${INSTALL_DIR}/bin/arm-unknown-linux-uclibcgnueabi-ar"
    target_ranlib="${INSTALL_DIR}/bin/arm-unknown-linux-uclibcgnueabi-ranlib"

    cat > "${config_file}" << EOF
# Rust build configuration for custom LLVM and ${ARCH} target
# Generated by bootstrap_rust.sh

[llvm]
# Use custom LLVM installation
# Note: llvm-config path is set via LLVM_CONFIG environment variable
download-ci-llvm = false
assertions = false
# Enable clang (boolean, not a path)
clang = true

[build]
# Build for host and target
host = ["x86_64-unknown-linux-gnu"]
target = ["x86_64-unknown-linux-gnu", "${TARGET_NAME}"]
# Use system Rust toolchain for bootstrap (matches version ${RUST_VERSION})
rustc = "${system_rustc}"
cargo = "${system_cargo}"
# Build extended tools (cargo, rustfmt, clippy, etc.)
extended = true
tools = ["cargo", "rustfmt", "clippy", "rustdoc", "src", "rust-analyzer-proc-macro-srv", "rust-analyzer"]

[install]
prefix = "${INSTALL_DIR}"
libdir = "lib"
docdir = "share/doc/rust"
mandir = "share/man"
# Set sysconfdir to a writable location (relative to prefix)
sysconfdir = "etc"

[target.x86_64-unknown-linux-gnu]
# Use native compiler for host build scripts
cc = "gcc"
cxx = "g++"
ar = "ar"
ranlib = "ranlib"

[target.${TARGET_NAME}]
# Linker configuration - use Clang wrapper for the target
linker = "${BUILD_DIR}/${CLANG_WRAPPER}"
# C compiler for target build scripts (but build scripts should use host compiler)
# These are only used when building for the target itself
cc = "${target_cc}"
cxx = "${target_cxx}"
ar = "${target_ar}"
ranlib = "${target_ranlib}"
# Linker flags to specify sysroot and target
# Clang needs these to find crti.o, crtn.o, and other C runtime files
# Note: Use RUSTFLAGS environment variable instead, as link-args format may vary

[rust]
# Optimize for size
optimize = true
debug = false
# Use LLVM LTO (string: "off", "thin", or "fat")
lto = "off"
# Codegen units
codegen-units = 1
# Deny warnings
deny-warnings = false
EOF

    log_info "Rust config.toml created: ${config_file}"

    # Ensure install directory is writable
    if [[ ! -w "${INSTALL_DIR}" ]]; then
        log_warn "Install directory is not writable. Attempting to fix permissions..."
        chmod -R u+w "${INSTALL_DIR}" 2>/dev/null || {
            log_warn "Could not fix permissions automatically. Trying with sudo..."
            sudo chmod -R u+w "${INSTALL_DIR}" || {
                log_error "Cannot write to install directory: ${INSTALL_DIR}"
                log_error "Please fix permissions manually: chmod -R u+w ${INSTALL_DIR}"
                exit 1
            }
        }
    fi
    return 0
}

# Build Rust
build_rust() {
    log_info "Building Rust (this will take 4-8 hours)..."

    cd "${RUST_SRC_DIR}"

    # Ensure cargo/rustc are in PATH
    export PATH="${HOME}/.cargo/bin:${PATH}"

    # Clear any cross-compilation environment variables that might interfere
    unset CC CXX AR RANLIB CFLAGS CXXFLAGS LDFLAGS
    unset TARGET_CC TARGET_CXX TARGET_AR TARGET_RANLIB
    # Ensure native compiler is used for host builds
    export CC="gcc"
    export CXX="g++"
    export AR="ar"
    export RANLIB="ranlib"

    # Set LLVM_CONFIG for custom LLVM (must be absolute path)
    export LLVM_CONFIG="${INSTALL_DIR}/bin/llvm-config"
    if [[ ! -f "${LLVM_CONFIG}" ]]; then
        log_error "LLVM_CONFIG not found: ${LLVM_CONFIG}"
        exit 1
    fi

    log_info "Starting Rust build process..."
    log_info "This is a long process. Progress will be logged to: ${BUILD_DIR}/rust_build.log"
    log_info "Using rustc: $(which rustc)"
    log_info "Using cargo: $(which cargo)"
    log_info "Using CC for host: ${CC}"
    log_info "Using CXX for host: ${CXX}"
    log_info "Using LLVM_CONFIG: ${LLVM_CONFIG}"

    # Create or verify Clang wrapper script that includes sysroot and target flags
    # This ensures Clang always has the right flags without affecting host builds
    local sysroot="${SYSROOT}"
    local clang_wrapper="${BUILD_DIR}/${CLANG_WRAPPER}"
    
    # Create ARMv5TE Clang wrapper
    cat > "${clang_wrapper}" << EOF
#!/bin/bash
# Clang wrapper for ARMv5TE target with sysroot
exec "${INSTALL_DIR}/bin/clang" \\
    --target=arm-unknown-linux-uclibcgnueabi \\
    --sysroot="${sysroot}" \\
    -march=armv5te \\
    -mfloat-abi=soft \\
    -mtune=arm926ej-s \\
    -L"${sysroot}/lib" \\
    -L"${sysroot}/usr/lib" \\
    "\$@"
EOF
    chmod +x "${clang_wrapper}"
    log_info "Created Clang wrapper: ${clang_wrapper}"

    # Update config.toml to use the wrapper (if not already set)
    if ! grep -q "linker = \"${clang_wrapper}\"" "${RUST_SRC_DIR}/config.toml"; then
        sed -i "s|linker = \".*\"|linker = \"${clang_wrapper}\"|" "${RUST_SRC_DIR}/config.toml" || \
        sed -i "s|linker = \".*\"|linker = \"${clang_wrapper}\"|" "${RUST_SRC_DIR}/config.toml"
        log_info "Updated config.toml to use Clang wrapper"
    fi

    # Build Rust compiler and std library for both host and target
    # Host is needed for build scripts, target is for cross-compilation
    # Targets are specified in config.toml, no need for --target flags
    python3 x.py build --stage 2 \
        2>&1 | tee "${BUILD_DIR}/rust_build.log" || {
        log_error "Rust build failed. Check log: ${BUILD_DIR}/rust_build.log"
        exit 1
    }

    log_info "Rust build completed successfully!"
    return 0
}

# Install Rust
install_rust() {
    log_info "Installing Rust..."

    cd "${RUST_SRC_DIR}"

    # Ensure cargo/rustc are in PATH
    export PATH="${HOME}/.cargo/bin:${PATH}"

    # Clear any cross-compilation environment variables that might interfere
    unset CC CXX AR RANLIB CFLAGS CXXFLAGS LDFLAGS
    unset TARGET_CC TARGET_CXX TARGET_AR TARGET_RANLIB
    # Ensure native compiler is used for host builds
    export CC="gcc"
    export CXX="g++"
    export AR="ar"
    export RANLIB="ranlib"

    # Set LLVM_CONFIG for custom LLVM (must be absolute path)
    export LLVM_CONFIG="${INSTALL_DIR}/bin/llvm-config"
    if [[ ! -f "${LLVM_CONFIG}" ]]; then
        log_error "LLVM_CONFIG not found: ${LLVM_CONFIG}"
        exit 1
    fi

    # Clang wrapper should already be created and configured in config.toml
    # No need to set RUSTFLAGS as the wrapper handles all the flags

    # Install Rust for all targets defined in config.toml
    # This installs compiler, stdlib, and extended tools (cargo, rustfmt, clippy)
    log_info "Installing Rust for all targets..."
    python3 x.py install --stage 2 \
        2>&1 | tee -a "${BUILD_DIR}/rust_install.log" || {
        log_error "Rust target installation failed. Check log: ${BUILD_DIR}/rust_install.log"
        exit 1
    }

    # Install rust-src component explicitly
    # rust-src is just source code (no compilation needed), so we copy it directly
    # This matches the bootstrap implementation in src/bootstrap/src/core/build_steps/dist.rs
    log_info "Installing rust-src component (copying library sources)..."
    local rust_src_dest="${INSTALL_DIR}/lib/rustlib/src/rust"
    
    # Create destination directory
    mkdir -p "${rust_src_dest}" || {
        log_error "Failed to create rust-src destination directory: ${rust_src_dest}"
        exit 1
    }
    
    # Copy library source files (matches bootstrap dist.rs implementation)
    if [[ -d "${RUST_SRC_DIR}/library" ]]; then
        log_info "Copying library sources from ${RUST_SRC_DIR}/library..."
        # Use rsync or cp with exclusions to match bootstrap behavior
        # Exclude: library/backtrace/crates, library/stdarch/Cargo.toml, etc.
        if command -v rsync >/dev/null 2>&1; then
            if rsync -a --exclude='backtrace/crates' \
                  --exclude='stdarch/Cargo.toml' \
                  --exclude='stdarch/crates/stdarch-verify' \
                  --exclude='stdarch/crates/intrinsic-test' \
                  "${RUST_SRC_DIR}/library/" "${rust_src_dest}/library/"; then
                log_info "Library sources copied successfully"
            else
                log_warn "rsync failed, falling back to cp"
                cp -r "${RUST_SRC_DIR}/library" "${rust_src_dest}/" || {
                    log_error "Failed to copy library sources"
                    exit 1
                }
                log_info "Library sources copied successfully"
            fi
        else
            log_warn "rsync not found, falling back to cp"
            cp -r "${RUST_SRC_DIR}/library" "${rust_src_dest}/" || {
                log_error "Failed to copy library sources"
                exit 1
            }
            log_info "Library sources copied successfully"
        fi
    else
        log_error "Library source directory not found: ${RUST_SRC_DIR}/library"
        exit 1
    fi
    
    # Copy libunwind source (needed for some std library dependencies)
    if [[ -d "${RUST_SRC_DIR}/src/llvm-project/libunwind" ]]; then
        log_info "Copying libunwind sources..."
        mkdir -p "${rust_src_dest}/src/llvm-project"
        cp -r "${RUST_SRC_DIR}/src/llvm-project/libunwind" "${rust_src_dest}/src/llvm-project/" || {
            log_warn "Failed to copy libunwind sources (non-critical)"
        }
    fi
    
    # Create version file
    local rust_version=$("${INSTALL_DIR}/bin/rustc" --version 2>/dev/null | cut -d' ' -f2 || echo "unknown")
    echo "${rust_version}" > "${rust_src_dest}/version" 2>/dev/null || true
    
    # Register rust-src in components file
    local components_file="${INSTALL_DIR}/lib/rustlib/components"
    if [[ -f "${components_file}" ]] && ! grep -q "rust-src" "${components_file}"; then
        echo "rust-src" >> "${components_file}"
        log_info "Registered rust-src in components file"
    fi
    
    log_info "rust-src component installed successfully at: ${rust_src_dest}"

    # Verify cargo was installed
    local cargo_path="${INSTALL_DIR}/bin/cargo"
    if [[ ! -f "${cargo_path}" ]]; then
        log_warn "cargo not found at ${cargo_path} after install"
        log_info "Attempting to install cargo explicitly..."

        # Try to install cargo explicitly
        python3 x.py install cargo --stage 2 \
            2>&1 | tee -a "${BUILD_DIR}/rust_install.log" || {
            log_warn "Explicit cargo install failed, checking if cargo exists elsewhere..."

            # Check if cargo was built but not installed
            local cargo_build="${RUST_SRC_DIR}/build/${TARGET_NAME}/stage2-tools-bin/cargo"
            if [[ -f "${cargo_build}" ]]; then
                log_info "Found cargo in build directory, copying to install location..."
                cp "${cargo_build}" "${cargo_path}"
                chmod +x "${cargo_path}"
                log_info "cargo copied successfully"
            else
                log_error "cargo not found in build directory either"
                log_error "Check ${BUILD_DIR}/rust_install.log for details"
                exit 1
            fi
        }
    fi

    # Verify rust-analyzer-proc-macro-srv was installed
    # It ships to libexec/ in most Rust installs (fallback: bin/)
    local proc_macro_srv=""
    if [[ -f "${INSTALL_DIR}/libexec/rust-analyzer-proc-macro-srv" ]]; then
        proc_macro_srv="${INSTALL_DIR}/libexec/rust-analyzer-proc-macro-srv"
    elif [[ -f "${INSTALL_DIR}/bin/rust-analyzer-proc-macro-srv" ]]; then
        proc_macro_srv="${INSTALL_DIR}/bin/rust-analyzer-proc-macro-srv"
    fi

    if [[ -n "${proc_macro_srv}" ]]; then
        log_info "rust-analyzer-proc-macro-srv installed: ${proc_macro_srv}"
    else
        log_warn "rust-analyzer-proc-macro-srv not found in libexec/ or bin/"
        log_warn "Proc-macro expansion in rust-analyzer will not work"
        log_warn "Check build log: ${BUILD_DIR}/rust_install.log"
    fi

    # Verify rust-analyzer was installed
    local rust_analyzer="${INSTALL_DIR}/bin/rust-analyzer"
    if [[ -f "${rust_analyzer}" ]]; then
        log_info "rust-analyzer installed: ${rust_analyzer}"
    else
        log_warn "rust-analyzer not found at: ${rust_analyzer}"
        log_warn "IDE language server features will not be available"
        log_warn "Check build log: ${BUILD_DIR}/rust_install.log"
    fi

    log_info "Rust installed successfully!"
    return 0
}

# Verify installation
verify_installation() {
    log_info "Verifying Rust installation..."

    local rustc_path="${INSTALL_DIR}/bin/rustc"
    if [[ ! -f "${rustc_path}" ]]; then
        log_error "rustc not found at expected location: ${rustc_path}"
        exit 1
    fi

    log_info "Testing rustc version..."
    "${rustc_path}" --version

    log_info "Testing target list..."
    if "${rustc_path}" --print target-list | grep -q "${TARGET_NAME}"; then
        log_info "Target ${TARGET_NAME} is available!"
    else
        log_warn "Target ${TARGET_NAME} not found in target list"
    fi

    # Verify rust-src component installation
    log_info "Verifying rust-src component..."
    local rust_src_path="${INSTALL_DIR}/lib/rustlib/src/rust"
    if [[ -d "${rust_src_path}" ]]; then
        log_info "rust-src component installed at: ${rust_src_path}"
        # Check for key library directories
        if [[ -d "${rust_src_path}/library/std" ]] && [[ -d "${rust_src_path}/library/core" ]]; then
            log_info "Standard library source code verified (std, core)"
        else
            log_warn "rust-src directory exists but library sources not found"
        fi
    else
        log_error "rust-src component not found at expected location: ${rust_src_path}"
        log_error "rust-analyzer will not be able to load standard library sources"
        exit 1
    fi

    # Verify components file includes rust-src
    local components_file="${INSTALL_DIR}/lib/rustlib/components"
    if [[ -f "${components_file}" ]]; then
        if grep -q "rust-src" "${components_file}"; then
            log_info "rust-src component registered in components file"
        else
            log_warn "rust-src not found in components file, but directory exists"
        fi
    fi

    # Verify rust-analyzer-proc-macro-srv
    local proc_macro_srv=""
    if [[ -f "${INSTALL_DIR}/libexec/rust-analyzer-proc-macro-srv" ]]; then
        proc_macro_srv="${INSTALL_DIR}/libexec/rust-analyzer-proc-macro-srv"
    elif [[ -f "${INSTALL_DIR}/bin/rust-analyzer-proc-macro-srv" ]]; then
        proc_macro_srv="${INSTALL_DIR}/bin/rust-analyzer-proc-macro-srv"
    fi

    if [[ -n "${proc_macro_srv}" ]]; then
        log_info "rust-analyzer-proc-macro-srv verified: ${proc_macro_srv}"
    else
        log_warn "rust-analyzer-proc-macro-srv not found — proc-macro expansion will be unavailable"
    fi

    # Verify rust-analyzer
    if [[ -f "${INSTALL_DIR}/bin/rust-analyzer" ]]; then
        log_info "rust-analyzer verified: ${INSTALL_DIR}/bin/rust-analyzer"
        "${INSTALL_DIR}/bin/rust-analyzer" --version 2>/dev/null || true
    else
        log_warn "rust-analyzer not found at: ${INSTALL_DIR}/bin/rust-analyzer"
    fi

    log_info "Rust verification completed"
    return 0
}

# Main execution
main() {
    log_info "Starting Rust bootstrap for ${ARCH}"
    log_info "Build directory: ${BUILD_DIR}"
    log_info "Install directory: ${INSTALL_DIR}"
    log_info "Rust version: ${RUST_VERSION}"
    log_info "Target: ${TARGET_NAME}"

    check_dependencies
    clone_rust
    add_target_spec
    create_rust_config
    build_rust
    install_rust
    verify_installation

    log_info "=========================================="
    log_info "Rust bootstrap completed successfully!"
    log_info "Installation location: ${INSTALL_DIR}"
    log_info "=========================================="
    log_info "rustc available at: ${INSTALL_DIR}/bin/rustc"
    log_info "cargo available at: ${INSTALL_DIR}/bin/cargo"
    log_info "Target ${TARGET_NAME} is ready to use!"
}

# Run main function
main "$@"
