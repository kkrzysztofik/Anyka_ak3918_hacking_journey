#!/bin/bash
# check-versions.sh — Compare pinned component versions against upstream latest.
#
# Purely informational: never exits non-zero. Network unavailability or any
# fetch error causes that component's check to be skipped with a warning.
#
# Usage (called from build.sh after sourcing common.sh):
#   source "${SCRIPTS_DIR}/check-versions.sh"
#   check_upstream_versions

# Guard against double-sourcing
[[ -n "${_CHECK_VERSIONS_SH:-}" ]] && return 0
readonly _CHECK_VERSIONS_SH=1

# ---------------------------------------------------------------------------
# _fetch — fetch URL with a short timeout; print stdout or return 1 on error
# ---------------------------------------------------------------------------
_fetch() {
    curl -sf --max-time 8 "$1" 2>/dev/null
}

# ---------------------------------------------------------------------------
# _ver_lt <a> <b> — return 0 (true) if version a < version b
# Uses sort -V for correct numeric comparison (1.9 < 1.10, etc.)
# ---------------------------------------------------------------------------
_ver_lt() {
    [[ "$1" != "$2" ]] && \
        [[ "$1" = "$(printf '%s\n%s' "$1" "$2" | sort -V | head -1)" ]]
}

# ---------------------------------------------------------------------------
# _check_one <name> <pinned> <latest>
# ---------------------------------------------------------------------------
_check_one() {
    local name="$1" pinned="$2" latest="$3"
    if [[ -z "${latest}" ]]; then
        echo "[VERSIONS] ${name}: pinned ${pinned} (upstream check failed — skipped)"
    elif _ver_lt "${pinned}" "${latest}"; then
        echo "[VERSIONS] ${name}: update available  ${pinned} → ${latest}"
    else
        echo "[VERSIONS] ${name}: up to date        ${pinned}"
    fi
}

# ---------------------------------------------------------------------------
# check_upstream_versions — main entry point
# ---------------------------------------------------------------------------
check_upstream_versions() {
    echo ""
    echo "=============================================="
    echo "Component version check (informational only)"
    echo "=============================================="

    # ── crosstool-NG ────────────────────────────────────────────────────────
    local latest_ctng
    latest_ctng=$(
        _fetch "https://api.github.com/repos/crosstool-ng/crosstool-ng/releases/latest" \
        | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'].replace('crosstool-ng-',''))" \
        2>/dev/null
    ) || true
    _check_one "crosstool-NG" "${CROSSTOOL_NG_VERSION}" "${latest_ctng}"

    # ── uClibc-ng ────────────────────────────────────────────────────────────
    # Authoritative source: the downloads directory index
    local latest_uclibc
    latest_uclibc=$(
        _fetch "https://downloads.uclibc-ng.org/releases/" \
        | grep -oP '(?<=href=")[\d]+\.[\d]+\.[\d]+(?=/")' \
        | sort -V | tail -1 \
        2>/dev/null
    ) || true
    _check_one "uClibc-ng" "${UCLIBC_NG_VERSION}" "${latest_uclibc}"

    # ── LLVM ─────────────────────────────────────────────────────────────────
    # Exclude release-candidates (tags with -rc)
    local latest_llvm
    latest_llvm=$(
        _fetch "https://api.github.com/repos/llvm/llvm-project/releases?per_page=20" \
        | python3 -c "
import sys, json, re
for r in json.load(sys.stdin):
    t = r['tag_name']
    if re.match(r'^llvmorg-[0-9]+\.[0-9]+\.[0-9]+\$', t):
        print(t.replace('llvmorg-', ''))
        break
" 2>/dev/null
    ) || true
    _check_one "LLVM/Clang" "${LLVM_VERSION}" "${latest_llvm}"

    # ── Rust ─────────────────────────────────────────────────────────────────
    # Stable releases are plain semver tags (no suffixes)
    local latest_rust
    latest_rust=$(
        _fetch "https://api.github.com/repos/rust-lang/rust/releases?per_page=20" \
        | python3 -c "
import sys, json, re
for r in json.load(sys.stdin):
    t = r['tag_name']
    if re.match(r'^[0-9]+\.[0-9]+\.[0-9]+\$', t):
        print(t)
        break
" 2>/dev/null
    ) || true
    _check_one "Rust" "${RUST_VERSION}" "${latest_rust}"

    # ── GDB ──────────────────────────────────────────────────────────────────
    # GNU FTP directory index is the authoritative source
    local latest_gdb
    latest_gdb=$(
        _fetch "https://ftp.gnu.org/gnu/gdb/?C=M&O=D" \
        | grep -oP 'gdb-\K[0-9]+\.[0-9]+(?:\.[0-9]+)?(?=\.tar\.xz)' \
        | grep -v rc \
        | sort -V | tail -1 \
        2>/dev/null
    ) || true
    _check_one "GDB" "${GDB_VERSION}" "${latest_gdb}"

    echo "=============================================="
    echo ""
}
