# CodeQL configuration

This directory holds the CodeQL config used to exclude vendor/third-party paths from analysis.

## Default Setup (GitHub UI)

If the repo uses **CodeQL Default Setup** (configured in GitHub → Settings → Code security and analysis):

1. After changing `codeql-config.yml`, refresh the configuration so new runs use it:
   - **Settings** → **Code security and analysis** → **CodeQL analysis** (Default) → **Edit**
   - Click **Save changes** (or disable then re-enable Default Setup).
2. Re-run the CodeQL check on your PR (e.g. re-run failed jobs or push an empty commit) to confirm alerts no longer come from ignored paths (e.g. `cross-compile/anyka_reference/**`).

## Advanced Setup (workflow file)

If Default Setup does not honor this config, you can switch to **Advanced Setup** using the workflow at `.github/workflows/codeql.yml`, which passes `config-file: ./.github/codeql/codeql-config.yml` to the CodeQL init action. In that case, disable Default Setup in the GitHub UI to avoid duplicate runs.
