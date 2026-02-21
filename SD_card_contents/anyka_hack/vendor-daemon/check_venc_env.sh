#!/bin/sh
# Check Anyka VENC/UIO environment for mmap(EINVAL) and no-output failures.
# Usage:
#   ./check_venc_env.sh [EXPECTED_BASE_HEX] [EXPECTED_LEN_HEX]
# Example:
#   ./check_venc_env.sh 0x20020000 0x10000

set -u

EXPECTED_BASE_HEX="${1:-0x20020000}"
EXPECTED_LEN_HEX="${2:-0x10000}"

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
LIB_DIR_DEFAULT="/mnt/anyka_hack/vendor-daemon/lib"
if [ -d "$LIB_DIR_DEFAULT" ]; then
    LIB_DIR="$LIB_DIR_DEFAULT"
else
    LIB_DIR="$SCRIPT_DIR/lib"
fi
BIN_PATH="$SCRIPT_DIR/vendor-daemon.bin"

FAILS=0
WARNS=0

note() {
    printf '%s\n' "$*"
}

ok() {
    printf '[OK] %s\n' "$*"
}

warn() {
    WARNS=$((WARNS + 1))
    printf '[WARN] %s\n' "$*"
}

fail() {
    FAILS=$((FAILS + 1))
    printf '[FAIL] %s\n' "$*"
}

section() {
    printf '\n=== %s ===\n' "$*"
}

hex_to_dec() {
    # shellcheck disable=SC2004
    printf '%u' "$(( $1 ))" 2>/dev/null
}

fmt_hex() {
    # print unsigned as 0xXXXXXXXX
    printf '0x%08x' "$1"
}

contains_text() {
    FILE="$1"
    TEXT="$2"
    if [ ! -f "$FILE" ]; then
        return 1
    fi
    if command -v strings >/dev/null 2>&1; then
        strings "$FILE" 2>/dev/null | grep -F "$TEXT" >/dev/null 2>&1
        return $?
    fi
    return 1
}

section "System"
note "Date: $(date 2>/dev/null || true)"
note "Kernel: $(uname -a 2>/dev/null || true)"
note "Expected VENC reg base: $EXPECTED_BASE_HEX"
note "Expected VENC reg len : $EXPECTED_LEN_HEX"

EXPECTED_BASE_DEC=$(hex_to_dec "$EXPECTED_BASE_HEX")
EXPECTED_LEN_DEC=$(hex_to_dec "$EXPECTED_LEN_HEX")
if [ -z "$EXPECTED_BASE_DEC" ] || [ -z "$EXPECTED_LEN_DEC" ]; then
    fail "Invalid expected hex values: base='$EXPECTED_BASE_HEX' len='$EXPECTED_LEN_HEX'"
    EXPECTED_BASE_DEC=0
    EXPECTED_LEN_DEC=0
fi
EXPECTED_END_DEC=$((EXPECTED_BASE_DEC + EXPECTED_LEN_DEC - 1))

section "Device Nodes"
if [ -c /dev/uio0 ]; then
    ok "/dev/uio0 exists"
else
    fail "/dev/uio0 missing"
fi

if [ -c /dev/ion ]; then
    ok "/dev/ion exists"
else
    warn "/dev/ion missing (some builds still require it for PMEM)"
fi

if [ -r /dev/uio0 ] && [ -w /dev/uio0 ]; then
    ok "/dev/uio0 is read/write"
else
    warn "Current user may not have read/write access to /dev/uio0"
fi

section "UIO Map Inspection"
if [ ! -d /sys/class/uio/uio0 ]; then
    fail "/sys/class/uio/uio0 missing"
else
    MAP_COUNT=0
    FOUND_EXACT=0
    FOUND_CONTAINING=0
    CONTAINING_MAP=""
    EXACT_SIZE_DEC=0

    for MAP_DIR in /sys/class/uio/uio0/maps/map*; do
        [ -d "$MAP_DIR" ] || continue
        MAP_COUNT=$((MAP_COUNT + 1))

        ADDR_HEX=$(cat "$MAP_DIR/addr" 2>/dev/null || echo "0x0")
        SIZE_HEX=$(cat "$MAP_DIR/size" 2>/dev/null || echo "0x0")
        OFF_HEX=$(cat "$MAP_DIR/offset" 2>/dev/null || echo "0x0")
        NAME=$(cat "$MAP_DIR/name" 2>/dev/null || echo "-")

        ADDR_DEC=$(hex_to_dec "$ADDR_HEX")
        SIZE_DEC=$(hex_to_dec "$SIZE_HEX")
        [ -z "$ADDR_DEC" ] && ADDR_DEC=0
        [ -z "$SIZE_DEC" ] && SIZE_DEC=0
        END_DEC=$((ADDR_DEC + SIZE_DEC - 1))

        note "$(basename "$MAP_DIR"): addr=$ADDR_HEX size=$SIZE_HEX offset=$OFF_HEX name=$NAME"

        if [ "$ADDR_DEC" -eq "$EXPECTED_BASE_DEC" ]; then
            FOUND_EXACT=1
            EXACT_SIZE_DEC=$SIZE_DEC
        fi

        if [ "$ADDR_DEC" -le "$EXPECTED_BASE_DEC" ] && [ "$EXPECTED_END_DEC" -le "$END_DEC" ]; then
            FOUND_CONTAINING=1
            CONTAINING_MAP=$(basename "$MAP_DIR")
        fi
    done

    if [ "$MAP_COUNT" -eq 0 ]; then
        fail "No /sys/class/uio/uio0/maps/map* entries found"
    else
        ok "Found $MAP_COUNT UIO map entries"
    fi

    if [ "$FOUND_EXACT" -eq 1 ]; then
        ok "Exact map for $EXPECTED_BASE_HEX exists"
        if [ "$EXACT_SIZE_DEC" -lt "$EXPECTED_LEN_DEC" ]; then
            warn "Exact map size $(fmt_hex "$EXACT_SIZE_DEC") is smaller than requested $(fmt_hex "$EXPECTED_LEN_DEC"); old libakuio may mmap(EINVAL)"
        else
            ok "Exact map size is >= expected len"
        fi
    else
        warn "No exact map starts at $EXPECTED_BASE_HEX"
    fi

    if [ "$FOUND_CONTAINING" -eq 1 ]; then
        ok "A containing map exists for requested range (map=$CONTAINING_MAP)"
    elif [ "$FOUND_EXACT" -eq 1 ] && [ "$EXACT_SIZE_DEC" -gt 0 ] && [ "$EXACT_SIZE_DEC" -lt "$EXPECTED_LEN_DEC" ]; then
        warn "Kernel exports a smaller exact VENC window ($(fmt_hex "$EXACT_SIZE_DEC")) than requested; userspace must cap mmap size to avoid EINVAL"
    else
        fail "No UIO map contains expected VENC range ${EXPECTED_BASE_HEX}..$(fmt_hex "$EXPECTED_END_DEC")"
    fi
fi

section "Kernel Memory View"
if [ -r /proc/iomem ]; then
    if grep -Ei "2002[0-9a-f]{4}|venc|video" /proc/iomem >/dev/null 2>&1; then
        ok "/proc/iomem has potential VENC/video entries"
        grep -Ei "2002[0-9a-f]{4}|venc|video" /proc/iomem 2>/dev/null | head -n 20
    else
        warn "No obvious VENC/video entry in /proc/iomem"
    fi
else
    warn "/proc/iomem not readable"
fi

section "Binary/Library Markers"
if [ -f "$BIN_PATH" ]; then
    ok "Binary found: $BIN_PATH"
else
    fail "Binary missing: $BIN_PATH"
fi

if [ -d "$LIB_DIR" ]; then
    note "Using lib dir: $LIB_DIR"
else
    warn "Lib dir not found: $LIB_DIR"
fi

NEEDED_LIBS=""
if [ -f "$BIN_PATH" ] && command -v readelf >/dev/null 2>&1; then
    NEEDED_LIBS=$(readelf -d "$BIN_PATH" 2>/dev/null \
        | sed -n 's/.*Shared library: \[\(.*\)\].*/\1/p')
    if [ -n "$NEEDED_LIBS" ]; then
        note "Binary dynamic deps: $(echo "$NEEDED_LIBS" | tr '\n' ' ')"
    else
        note "Binary dynamic deps: (none parsed)"
    fi
fi

case "$NEEDED_LIBS" in
    *libmpi_venc.so*)
        if [ -f "$LIB_DIR/libmpi_venc.so" ]; then
            if contains_text "$LIB_DIR/libmpi_venc.so" "set_iframe: locking enc_handle->lock"; then
                ok "libmpi_venc.so contains extended set_iframe lock logs"
            else
                warn "libmpi_venc.so missing new set_iframe lock markers"
            fi
        else
            fail "Binary needs libmpi_venc.so but file not found in $LIB_DIR"
        fi
        ;;
    *)
        note "libmpi_venc.so not listed in binary NEEDED entries (likely static-link path); skipping .so marker check"
        ;;
esac

case "$NEEDED_LIBS" in
    *libakuio.so*)
        if [ -f "$LIB_DIR/libakuio.so" ]; then
            if contains_text "$LIB_DIR/libakuio.so" "kernel_map_size=0x%x" \
                || contains_text "$LIB_DIR/libakuio.so" "/sys/class/uio/uio0/maps/map%u/size" \
                || contains_text "$LIB_DIR/libakuio.so" "capping mmap size"; then
                ok "libakuio.so contains short-UIO-map compatibility logic"
            else
                warn "libakuio.so appears old (no short-UIO-map compatibility marker)"
            fi
        else
            fail "Binary needs libakuio.so but file not found in $LIB_DIR"
        fi
        ;;
    *)
        note "libakuio.so not listed in binary NEEDED entries (likely static-link path); skipping .so marker check"
        ;;
esac

section "Summary"
note "FAILS=$FAILS WARNS=$WARNS"
if [ "$FAILS" -gt 0 ]; then
    fail "Environment NOT ready for stable VENC startup"
    exit 2
fi
if [ "$WARNS" -gt 0 ]; then
    warn "Environment usable but has warnings"
    exit 0
fi
ok "Environment looks good for VENC register mapping/startup"
exit 0
