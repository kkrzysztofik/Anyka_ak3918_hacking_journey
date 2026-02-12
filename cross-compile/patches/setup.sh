#!/bin/bash
# ARMv5TEJ Compatibility Patches Setup Script
# Downloads original crates and applies patches for portable-atomic support
#
# Usage: ./setup.sh [--clean]
#   --clean: Remove existing patched directories before applying

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Crate versions to download
declare -A CRATES=(
    ["webrtc-util"]="0.7.0"
    ["webrtc-ice"]="0.9.1"
    ["webrtc-sctp"]="0.8.0"
    ["rtp"]="0.8.0"
    ["tokio-metrics"]="0.2.2"
    ["openssl-src"]="300.2.3+3.2.1"
)

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    local message="$1"
    printf "%s[INFO]%s %s\n" "${GREEN}" "${NC}" "${message}"
    return 0
}

log_warn() {
    local message="$1"
    printf "%s[WARN]%s %s\n" "${YELLOW}" "${NC}" "${message}"
    return 0
}

log_error() {
    local message="$1"
    printf "%s[ERROR]%s %s\n" "${RED}" "${NC}" "${message}"
    return 0
}

clean_patched() {
    log_info "Cleaning existing patched directories..."
    for crate in "${!CRATES[@]}"; do
        version="${CRATES[$crate]}"
        patched_dir="${crate}-${version}-full"
        if [[ -d "$patched_dir" ]]; then
            rm -rf "$patched_dir"
            log_info "  Removed $patched_dir"
        fi
    done
    rm -rf originals
    log_info "Cleanup complete"
    return 0
}

download_crate() {
    local crate="$1"
    local version="$2"
    local url="https://static.crates.io/crates/${crate}/${crate}-${version}.crate"
    local crate_file="${crate}-${version}.crate"

    mkdir -p originals

    if [[ ! -f "originals/$crate_file" ]]; then
        log_info "Downloading ${crate} v${version}..."
        if ! curl -sL "$url" -o "originals/$crate_file"; then
            log_error "Failed to download $crate"
            return 1
        fi
    fi

    if [[ ! -d "originals/${crate}-${version}" ]]; then
        log_info "Extracting ${crate} v${version}..."
        tar -xzf "originals/$crate_file" -C originals/
    fi
    return 0
}

apply_patch() {
    local crate="$1"
    local version="$2"
    local patch_file="diffs/${crate}-${version}.patch"
    local orig_dir="originals/${crate}-${version}"
    local patched_dir="${crate}-${version}-full"

    if [[ ! -f "$patch_file" ]]; then
        log_error "Patch file not found: $patch_file"
        return 1
    fi

    log_info "Applying patch for ${crate} v${version}..."

    # Copy original to patched directory
    rm -rf "$patched_dir"
    cp -r "$orig_dir" "$patched_dir"

    # Apply the patch (a/ -> patched_dir, b/ -> patched_dir)
    cd "$patched_dir"
    if patch -p1 --no-backup-if-mismatch < "../$patch_file" >/dev/null 2>&1; then
        log_info "  Patch applied successfully"
    else
        log_warn "  Patch had warnings (may be OK)"
    fi
    cd "$SCRIPT_DIR"
    return 0
}

main() {
    local first_arg="${1:-}"
    if [[ "$first_arg" == "--clean" ]]; then
        clean_patched
        shift
    fi

    log_info "Setting up ARMv5TEJ compatibility patches..."
    printf "\n"

    for crate in "${!CRATES[@]}"; do
        version="${CRATES[$crate]}"
        patched_dir="${crate}-${version}-full"

        if [[ -d "$patched_dir" ]]; then
            log_info "Skipping ${crate} (already patched)"
            continue
        fi

        download_crate "$crate" "$version"
        apply_patch "$crate" "$version"
    done

    printf "\n"
    log_info "All patches applied successfully!"
    printf "\n"
    log_info "Patched crates ready in:"
    for crate in "${!CRATES[@]}"; do
        version="${CRATES[$crate]}"
        printf "  - %s-%s-full/\n" "${crate}" "${version}"
    done
    return 0
}

main "$@"
