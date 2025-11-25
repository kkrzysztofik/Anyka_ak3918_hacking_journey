#!/bin/sh
#
# run_xiu.sh - Wrapper script for xiu streaming server
#
# Description:
#   This script sets up the library path and executes the xiu binary
#   with all passed arguments. It follows the pattern used by other
#   tools in the anyka_hack system.
#
# Usage:
#   ./run_xiu.sh [xiu arguments...]
#
# Dependencies:
#   - common.sh (sourced automatically)
#   - init_logs.sh (sourced automatically)
#   - xiu binary in the same directory
#   - Shared lib directory at /mnt/anyka_hack/lib with required shared libraries
#
# Author: Anyka Hack Project
# Version: 1.0

LOG_FILE=xiu.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

# Fallback log function if common.sh wasn't sourced or log function not available
# Use a simple test that works in busybox sh
_log_defined=0
if (log INFO test 2>/dev/null) >/dev/null 2>&1; then
  _log_defined=1
fi
if [ "$_log_defined" = "0" ]; then
  log() {
    local level="$1"
    shift
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [${level}] $*" >&2
  }
fi
unset _log_defined

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
XIU_BINARY="${SCRIPT_DIR}/xiu"
# Use shared lib directory instead of local one
SHARED_LIB_DIR="/mnt/anyka_hack/lib"

# Validate xiu binary exists
if [ ! -f "${XIU_BINARY}" ]; then
  log ERROR "xiu binary not found at: ${XIU_BINARY}"
  exit 1
fi

# Validate shared lib directory exists
if [ ! -d "${SHARED_LIB_DIR}" ]; then
  log ERROR "Shared library directory not found at: ${SHARED_LIB_DIR}"
  exit 1
fi

# The binary is configured to use /mnt/anyka_hack/lib/ld-uClibc.so.1 as the dynamic linker
# Verify the dynamic linker exists in the shared lib directory
if [ ! -f "${SHARED_LIB_DIR}/ld-uClibc.so.1" ]; then
  log ERROR "Dynamic linker not found at: ${SHARED_LIB_DIR}/ld-uClibc.so.1"
  exit 1
fi

log INFO "Starting xiu with shared library path: ${SHARED_LIB_DIR}"
log DEBUG "Using dynamic linker: ${SHARED_LIB_DIR}/ld-uClibc.so.1"
log DEBUG "Executing: ${XIU_BINARY} $*"

# Set LD_LIBRARY_PATH to include shared lib directory
export LD_LIBRARY_PATH="${SHARED_LIB_DIR}:${LD_LIBRARY_PATH}"

# Execute xiu - it should use the dynamic linker from /mnt/anyka_hack/lib/ld-uClibc.so.1
exec "${XIU_BINARY}" "$@"
