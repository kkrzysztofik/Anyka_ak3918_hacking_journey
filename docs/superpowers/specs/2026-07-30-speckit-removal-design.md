# Design: Remove SpecKit tooling (keep artifacts)

**Date:** 2026-07-30  
**Status:** Approved  
**Approach:** A — delete engine + all SpecKit commands/prompts; keep existing `docs/specs` artifacts.

## Goals

1. Remove SpecKit so agents cannot invoke specify/plan/tasks/implement workflows.
2. Preserve existing feature specs under `docs/specs/**` and `docs/archive/specs/**`.
3. Do not rewrite historical SpecKit mentions inside those docs.

## Delete

| Path | Notes |
|------|--------|
| `.specify/` | Engine: memory, scripts, templates |
| `.cursor/commands/speckit.*` | 9 Cursor commands |
| `.github/prompts/speckit.*` | 8 Copilot prompts |
| `.gemini/commands/speckit.*` | Gemini ports (already absent on disk) |

## Keep

- `docs/specs/**`, `docs/archive/specs/**`
- Non-SpecKit prompts/commands (e.g. `.cursor/commands/github-action.mdc`, `.github/prompts/code-review.prompt.md`, …)
- `docs/superpowers/specs/**` (unrelated design docs)

## Authorization

User: “approved — delete those files” (2026-07-30).
