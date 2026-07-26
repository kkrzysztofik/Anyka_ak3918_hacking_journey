# Loaded by integrated terminal profile "Anyka (setenv)" (see .vscode/settings.json).
# Resolves repo root from this file's location and sources the canonical env script.
if [[ -f "${HOME}/.bashrc" ]]; then
  # shellcheck source=/dev/null
  source "${HOME}/.bashrc"
fi
_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=/dev/null
source "${_repo_root}/setenv.sh"
unset _repo_root
