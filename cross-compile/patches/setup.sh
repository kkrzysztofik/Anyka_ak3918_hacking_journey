#!/bin/bash
# ARMv5TEJ Compatibility Patches Setup Script
# Downloads original crates and applies patches for portable-atomic support
#
# Usage: ./setup.sh [--clean]
#   --clean: Remove existing patched directories before applying

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "$(cd "${SCRIPT_DIR}/../.." && pwd)/scripts/common.sh"
cd "$SCRIPT_DIR"

# Crate versions to download
declare -A CRATES=(
    ["openssl-src"]="300.2.3+3.2.1"
    ["tower-http"]="0.7.0"
)

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

patch_stamp() {
    local patched_dir="$1"
    printf '%s/.patch-sha256' "$patched_dir"
}

patch_sha256() {
    local patch_file="$1"
    sha256sum "$patch_file" | awk '{print $1}'
}

# True when *-full/ was produced from the current diffs/*.patch contents.
is_patch_fresh() {
    local patched_dir="$1"
    local patch_file="$2"
    local stamp_file
    stamp_file="$(patch_stamp "$patched_dir")"
    [[ -d "$patched_dir" && -f "$patch_file" && -f "$stamp_file" ]] || return 1
    [[ "$(cat "$stamp_file")" == "$(patch_sha256 "$patch_file")" ]]
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
    if patch -p1 --no-backup-if-mismatch < "../$patch_file"; then
        log_info "  Patch applied successfully"
        patch_sha256 "../$patch_file" > .patch-sha256
        cd "$SCRIPT_DIR"
        return 0
    fi
    log_error "  Patch failed for ${crate} v${version}"
    cd "$SCRIPT_DIR"
    rm -rf "$patched_dir"
    return 1
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

        patch_file="diffs/${crate}-${version}.patch"
        if is_patch_fresh "$patched_dir" "$patch_file"; then
            log_info "Skipping ${crate} (already patched)"
            continue
        fi
        if [[ -d "$patched_dir" ]]; then
            log_info "Refreshing ${crate} (patch file changed)"
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
