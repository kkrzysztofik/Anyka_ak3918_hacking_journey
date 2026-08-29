#!/usr/bin/env bash
# shellcheck disable=SC2034
# scripts/common.sh — Shared host-side utilities for Anyka repo shell scripts.
#
# Sourcing (from a script next to this file):
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   # shellcheck source=scripts/common.sh
#   source "${SCRIPT_DIR}/common.sh"
#
# From nested paths (e.g. cross-compile/foo/scripts/bar.sh), use a path to this file:
#   source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)/scripts/common.sh"

[[ -n "${_ANYKA_SCRIPTS_COMMON_SH:-}" ]] && return 0
readonly _ANYKA_SCRIPTS_COMMON_SH=1

# ── Repo root (AGENTS.md marker) ────────────────────────────────────────────
anyka_resolve_repo_root() {
  local start_dir
  if [[ -n "${BASH_SOURCE[1]:-}" ]]; then
    start_dir="$(cd "$(dirname "${BASH_SOURCE[1]}")" && pwd)"
  else
    start_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  fi
  local d="$start_dir"
  while [[ "${d}" != "/" ]]; do
    if [[ -f "${d}/AGENTS.md" ]]; then
      printf '%s\n' "${d}"
      return 0
    fi
    d="$(dirname "${d}")"
  done
  return 1
}

if [[ -z "${ANYKA_REPO_ROOT:-}" ]]; then
  if ! ANYKA_REPO_ROOT="$(anyka_resolve_repo_root)"; then
    echo "[ERROR] Could not find Anyka repo root (no AGENTS.md above caller script)." >&2
    return 1 2>/dev/null || exit 1
  fi
  readonly ANYKA_REPO_ROOT
fi

# ── Toolchain paths (aligned with setenv.sh) ────────────────────────────────
ANYKA_TOOLCHAIN_BIN="${ANYKA_REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng/bin"
ANYKA_CARGO="${ANYKA_TOOLCHAIN_BIN}/cargo"
ANYKA_RUSTC="${ANYKA_TOOLCHAIN_BIN}/rustc"
ANYKA_RUSTDOC="${ANYKA_TOOLCHAIN_BIN}/rustdoc"
export ANYKA_REPO_ROOT ANYKA_TOOLCHAIN_BIN ANYKA_CARGO ANYKA_RUSTC ANYKA_RUSTDOC

# ── Vendored toolchain first on PATH ────────────────────────────────────────
# Exporting ANYKA_CARGO is not enough on its own: cargo resolves `rustc` from
# PATH, so a rustup shim in ~/.cargo/bin wins and compiles against artifacts
# built by the vendored rustc. The result is an opaque
#   E0514: found crate `x` compiled by an incompatible version of rustc
# in whatever dependency happens to build first.
#
# setenv.sh already does this for interactive shells. Doing it here too makes
# every script that sources this file self-sufficient, so forgetting to
# `source setenv.sh` is no longer a way to get a confusing compiler error.
#
# No-op when the toolchain is absent (e.g. before it has been built) — the
# toolchain builder has its own toolchain/toolchain-builder/scripts/common.sh
# and is unaffected by this file.
anyka_use_vendored_toolchain() {
  [[ -d "${ANYKA_TOOLCHAIN_BIN}" ]] || return 0

  # Idempotent: drop any existing occurrence before prepending.
  local p="${PATH}"
  p="${p//:${ANYKA_TOOLCHAIN_BIN}/}"
  p="${p//${ANYKA_TOOLCHAIN_BIN}:/}"
  p="${p#${ANYKA_TOOLCHAIN_BIN}}"
  export PATH="${ANYKA_TOOLCHAIN_BIN}:${p}"

  # Cargo resolves subcommands from $CARGO_HOME/bin *before* PATH. rustup
  # installs cargo-clippy / cargo-fmt proxies there, and those drag in rustup's
  # rustc — the same E0514 clash by another route. Use a project-local
  # CARGO_HOME that shares the user's crate cache but has an empty bin/, so the
  # vendored cargo-* on PATH wins. Set ANYKA_KEEP_CARGO_HOME=1 to opt out.
  if [[ -z "${ANYKA_KEEP_CARGO_HOME:-}" ]]; then
    local cargo_home="${ANYKA_REPO_ROOT}/toolchain/cargo-home"
    mkdir -p "${cargo_home}/bin"
    local sub
    for sub in registry git cache; do
      if [[ -d "${HOME}/.cargo/${sub}" && ! -e "${cargo_home}/${sub}" ]]; then
        ln -sfn "${HOME}/.cargo/${sub}" "${cargo_home}/${sub}"
      fi
    done
    export CARGO_HOME="${cargo_home}"
  fi
}

anyka_use_vendored_toolchain

# ── Colors ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# ── Logging (errors to stderr) ───────────────────────────────────────────────
log_info() {
  echo -e "${GREEN}[INFO]${NC} $*"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $*" >&2
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $*" >&2
}

log_success() {
  echo -e "${GREEN}[PASS]${NC} $*"
}

# Optional alias used by some scripts (e.g. test_video_latency)
log_step() {
  echo -e "${BLUE}[STEP]${NC} $*"
}

# ── Dependencies ────────────────────────────────────────────────────────────
# anyka_check_commands <cmd> [cmd ...] — exit 1 if any command is missing from PATH.
anyka_check_commands() {
  local missing=()
  local cmd
  for cmd in "$@"; do
    if ! command -v "${cmd}" &>/dev/null; then
      missing+=("${cmd}")
    fi
  done
  if [[ ${#missing[@]} -ne 0 ]]; then
    log_error "Missing dependencies: ${missing[*]}"
    log_info "Install with: sudo apt-get install -y ${missing[*]}"
    exit 1
  fi
}

# anyka_require_vendored_cargo — exit 1 if toolchain cargo is missing or not executable.
anyka_require_vendored_cargo() {
  if [[ ! -x "${ANYKA_CARGO}" ]]; then
    log_error "Toolchain cargo not found or not executable: ${ANYKA_CARGO}"
    log_info "Ensure the vendored toolchain is installed and run from the Anyka repo."
    exit 1
  fi
}

# anyka_require_sound_clips <onvif_dir> — exit 1 unless the committed event clips
# exist under <onvif_dir>/sounds/. Filenames match [sound.events] in config.toml.
# Clips are regenerated with scripts/make_speech.py (ElevenLabs + ffmpeg), never
# during a normal SD/bundle build.
anyka_require_sound_clips() {
  local onvif_dir="$1"
  local sounds_dir="${onvif_dir}/sounds"
  local clip
  local missing=()
  for clip in boot.raw alert.raw ok.raw upgrade.raw; do
    if [[ ! -f "${sounds_dir}/${clip}" ]]; then
      missing+=("${clip}")
    fi
  done
  if [[ ${#missing[@]} -ne 0 ]]; then
    log_error "Missing sound clips under ${sounds_dir}/: ${missing[*]}"
    log_info "Regenerate with: ELEVENLABS_API_KEY=… python3 scripts/make_speech.py"
    exit 1
  fi
}
