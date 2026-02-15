#!/bin/sh
#
# profile_onvif.sh - Capture strace + perf profile for onvif-rust on Anyka device
#
# Usage examples:
#   /mnt/anyka_hack/profile_onvif.sh
#   /mnt/anyka_hack/profile_onvif.sh --duration 45 --freq 49
#   /mnt/anyka_hack/profile_onvif.sh --pid 1234 --out-dir /mnt/logs/profiles

set -eu

PERF_BIN="/mnt/anyka_hack/bin/perf"
STRACE_BIN="/mnt/anyka_hack/bin/strace"
PROCESS_NAME="onvif-rust"
PTRACE_SCOPE_PATH="/proc/sys/kernel/yama/ptrace_scope"
PID=""
DURATION="30"
FREQ="49"
OUT_ROOT="/mnt/logs/profiles"
STRACE_PID=""
STRACE_EXIT_CODE=""
PERF_SUPPORTED="true"

usage() {
  cat <<EOF
Usage: $(basename "$0") [options]

Options:
  --pid <pid>            Attach to explicit PID
  --process <name>       Process name for pidof lookup (default: ${PROCESS_NAME})
  --duration <seconds>   Capture duration (default: ${DURATION})
  --freq <hz>            perf sample frequency (default: ${FREQ})
  --out-dir <path>       Output directory (default: ${OUT_ROOT})
  -h, --help             Show this help
EOF
}

is_positive_int() {
  case "$1" in
    ''|*[!0-9]*)
      return 1
      ;;
    0)
      return 1
      ;;
    *)
      return 0
      ;;
  esac
}

cleanup() {
  if [ -n "${STRACE_PID}" ] && kill -0 "${STRACE_PID}" 2>/dev/null; then
    kill -INT "${STRACE_PID}" 2>/dev/null || true
    wait "${STRACE_PID}" 2>/dev/null || true
  fi
}

print_attach_diagnostics() {
  echo "strace attach diagnostics:" >&2

  if [ -s "${RUN_DIR}/strace.stderr.log" ]; then
    echo "  strace_stderr:" >&2
    sed -n '1,6p' "${RUN_DIR}/strace.stderr.log" | sed 's/^/    /' >&2
  else
    echo "  strace_stderr: <empty>" >&2
  fi

  if [ -s "${RUN_DIR}/strace.stdout.log" ]; then
    echo "  strace_stdout:" >&2
    sed -n '1,6p' "${RUN_DIR}/strace.stdout.log" | sed 's/^/    /' >&2
  else
    echo "  strace_stdout: <empty>" >&2
  fi

  self_uid="$(id -u 2>/dev/null || echo unknown)"
  target_uid="$(awk '/^Uid:/{print $2}' "/proc/${PID}/status" 2>/dev/null || echo unknown)"
  echo "  collector_uid=${self_uid} target_uid=${target_uid}" >&2
  if kill -0 "${PID}" 2>/dev/null; then
    echo "  target_pid_alive=yes pid=${PID}" >&2
  else
    echo "  target_pid_alive=no pid=${PID}" >&2
    current_pid="$(pidof "${PROCESS_NAME}" 2>/dev/null | awk '{print $1}')"
    if [ -n "${current_pid}" ]; then
      echo "  target_pid_now=${current_pid} (process restarted)" >&2
    fi
  fi

  if [ -r "${PTRACE_SCOPE_PATH}" ]; then
    ptrace_scope="$(cat "${PTRACE_SCOPE_PATH}" 2>/dev/null || echo unknown)"
    echo "  yama.ptrace_scope=${ptrace_scope}" >&2
    if [ "${ptrace_scope}" != "0" ]; then
      echo "  hint: as root, run: echo 0 > ${PTRACE_SCOPE_PATH}" >&2
    fi
  else
    echo "  yama.ptrace_scope: unavailable on this kernel" >&2
  fi

  if [ -n "${STRACE_EXIT_CODE}" ]; then
    echo "  strace_exit_code=${STRACE_EXIT_CODE}" >&2
    case "${STRACE_EXIT_CODE}" in
      ''|*[!0-9]*)
        ;;
      *)
        if [ "${STRACE_EXIT_CODE}" -ge 128 ]; then
          echo "  strace_exit_signal=$((STRACE_EXIT_CODE - 128))" >&2
        fi
        ;;
    esac
  fi

  if command -v dmesg >/dev/null 2>&1; then
    dmesg_tail="$(dmesg 2>/dev/null | tail -n 80 | grep -Ei 'strace|segfault|illegal|trap|fault|ptrace' || true)"
    if [ -n "${dmesg_tail}" ]; then
      echo "  kernel_log_hints:" >&2
      echo "${dmesg_tail}" | sed -n '1,10p' | sed 's/^/    /' >&2
    fi
  fi

  if grep -q "Operation not permitted" "${RUN_DIR}/strace.stderr.log" 2>/dev/null; then
    echo "  hint: ptrace denied (permissions/capabilities). Run script as root and retry." >&2
  elif grep -q "No such process" "${RUN_DIR}/strace.stderr.log" 2>/dev/null; then
    echo "  hint: target PID disappeared; retry with fresh pidof output." >&2
  elif [ "${STRACE_EXIT_CODE}" = "139" ]; then
    echo "  hint: strace crashed (SIGSEGV). Likely binary/runtime mismatch on device." >&2
    echo "  hint: test directly: /mnt/anyka_hack/bin/strace.bin -V" >&2
    echo "  hint: rebuild static strace on host: ./scripts/third_party/build_strace.sh --link-mode static" >&2
  fi
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --pid)
      PID="$2"
      shift 2
      ;;
    --process)
      PROCESS_NAME="$2"
      shift 2
      ;;
    --duration)
      DURATION="$2"
      shift 2
      ;;
    --freq)
      FREQ="$2"
      shift 2
      ;;
    --out-dir)
      OUT_ROOT="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if ! is_positive_int "${DURATION}"; then
  echo "ERROR: --duration must be a positive integer" >&2
  exit 1
fi

if ! is_positive_int "${FREQ}"; then
  echo "ERROR: --freq must be a positive integer" >&2
  exit 1
fi

if [ -z "${PID}" ]; then
  PID="$(pidof "${PROCESS_NAME}" 2>/dev/null | awk '{print $1}')"
fi

if [ -z "${PID}" ]; then
  echo "ERROR: could not find process PID (name: ${PROCESS_NAME})" >&2
  exit 1
fi

if ! kill -0 "${PID}" 2>/dev/null; then
  echo "ERROR: target PID is not running: ${PID}" >&2
  exit 1
fi

if [ ! -x "${PERF_BIN}" ]; then
  echo "ERROR: perf binary not executable: ${PERF_BIN}" >&2
  exit 1
fi

if [ ! -x "${STRACE_BIN}" ]; then
  echo "ERROR: strace binary not executable: ${STRACE_BIN}" >&2
  exit 1
fi

if ! mkdir -p "${OUT_ROOT}" 2>/dev/null; then
  OUT_ROOT="/mnt/tmp/profiles"
  mkdir -p "${OUT_ROOT}"
  echo "WARN: falling back to ${OUT_ROOT}" >&2
fi

TS="$(date -u +%Y%m%dT%H%M%SZ 2>/dev/null || date +%Y%m%dT%H%M%S)"
RUN_DIR="${OUT_ROOT}/onvif_profile_${TS}_pid${PID}"
mkdir -p "${RUN_DIR}"

trap cleanup INT TERM EXIT

{
  echo "timestamp_utc=${TS}"
  echo "target_pid=${PID}"
  echo "process_name=${PROCESS_NAME}"
  echo "duration_seconds=${DURATION}"
  echo "perf_frequency_hz=${FREQ}"
  echo "hostname=$(hostname 2>/dev/null || echo unknown)"
  echo "kernel=$(uname -a 2>/dev/null || echo unknown)"
} > "${RUN_DIR}/meta.txt"

if [ -r "/proc/${PID}/cmdline" ]; then
  tr '\000' ' ' < "/proc/${PID}/cmdline" > "${RUN_DIR}/cmdline.txt" 2>/dev/null || true
fi

ps > "${RUN_DIR}/ps_snapshot.txt" 2>/dev/null || true
cat /proc/loadavg > "${RUN_DIR}/loadavg.txt" 2>/dev/null || true
cat /proc/meminfo > "${RUN_DIR}/meminfo.txt" 2>/dev/null || true

echo "Starting strace on PID ${PID}..."
"${STRACE_BIN}" -tt -T -f -p "${PID}" -o "${RUN_DIR}/onvif.strace" \
  > "${RUN_DIR}/strace.stdout.log" 2> "${RUN_DIR}/strace.stderr.log" &
STRACE_PID="$!"
sleep 1
if ! kill -0 "${STRACE_PID}" 2>/dev/null; then
  set +e
  wait "${STRACE_PID}" 2>/dev/null
  STRACE_EXIT_CODE="$?"
  set -e
  echo "ERROR: strace failed to attach." >&2
  echo "Run directory: ${RUN_DIR}" >&2
  print_attach_diagnostics
  exit 1
fi

echo "Recording perf for ${DURATION}s..."
set +e
"${PERF_BIN}" record -F "${FREQ}" -g -p "${PID}" -o "${RUN_DIR}/perf.data" -- sleep "${DURATION}" \
  > "${RUN_DIR}/perf_record.log" 2>&1
perf_rc="$?"
set -e
if [ "${perf_rc}" -ne 0 ]; then
  if grep -Eqi 'sys_perf_event_open\(\).*Function not implemented|No CONFIG_PERF_EVENTS|CONFIG_PERF_EVENTS=y|perf_event_open.*Function not implemented' "${RUN_DIR}/perf_record.log"; then
    PERF_SUPPORTED="false"
    {
      echo "perf_supported=false"
      echo "perf_exit_code=${perf_rc}"
      echo "reason=kernel perf_event_open not implemented (likely missing CONFIG_PERF_EVENTS)"
    } > "${RUN_DIR}/perf_unavailable.txt"
    echo "WARN: perf sampling unsupported by running kernel. Falling back to strace-only capture for ${DURATION}s." >&2
    sleep "${DURATION}"
  else
    echo "ERROR: perf record failed. See ${RUN_DIR}/perf_record.log" >&2
    echo "perf_exit_code=${perf_rc}" >&2
    if [ "${perf_rc}" = "139" ] || grep -Eqi 'segmentation fault|illegal instruction|sigsegv|sigill' "${RUN_DIR}/perf_record.log"; then
      echo "hint: perf crashed. Rebuild static perf on host: ./scripts/third_party/build_perf.sh --link-mode static" >&2
    fi
    exit 1
  fi
fi

if kill -0 "${STRACE_PID}" 2>/dev/null; then
  kill -INT "${STRACE_PID}" 2>/dev/null || true
  wait "${STRACE_PID}" 2>/dev/null || true
  STRACE_PID=""
fi

if [ "${PERF_SUPPORTED}" = "true" ] && [ -f "${RUN_DIR}/perf.data" ]; then
  echo "Generating perf text report..."
  "${PERF_BIN}" report --stdio -i "${RUN_DIR}/perf.data" \
    > "${RUN_DIR}/perf_report.txt" 2> "${RUN_DIR}/perf_report.err" || true
else
  echo "Skipping perf report: running kernel does not support perf sampling." \
    > "${RUN_DIR}/perf_report.err"
fi

ARCHIVE_PATH="${RUN_DIR}.tar.gz"
(cd "${OUT_ROOT}" && tar -czf "$(basename "${ARCHIVE_PATH}")" "$(basename "${RUN_DIR}")")

echo "Profile capture complete."
echo "Run directory: ${RUN_DIR}"
echo "Archive: ${ARCHIVE_PATH}"
