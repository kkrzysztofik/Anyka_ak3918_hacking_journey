# Design: Remove UBS, beads, and distill

**Date:** 2026-07-30  
**Status:** Approved  
**Approach:** Surgical purge of live agent config + data; leave historical plan docs alone.

## Goals

1. Stop agents from being instructed to use UBS, beads/`br`, or distill.
2. Delete repo data and rule files for those tools.
3. Uninstall global binaries on this machine (`ubs`, `br`; distill already absent).
4. Do **not** introduce a replacement issue tracker.

## Non-goals

- Rewriting historical mentions in `docs/plans/**` or `docs/superpowers/plans/**`.
- Removing unrelated false positives (“subsystem”, “stubs”).
- Changing SpecKit, skills, or agent rosters (separate inventory work).

## Delete (repo)

| Path | Reason |
|------|--------|
| `.beads/` | beads issue DB/JSONL/config |
| `.cursor/rules/beads.mdc` | beads Cursor rule |
| `.cursor/rules/distill.mdc` | distill Cursor rule |
| `.cursor/rules/ubs.md` | UBS Cursor rule |
| `.codex/rules/ubs.md` | UBS Codex rule |
| `.opencode/rules` | UBS-only OpenCode rules file |
| `.gemini/rules` | UBS-only Gemini rules file |
| `.opencode/commands/br-ready.md` | beads OpenCode command |
| `.claude/hooks/on-file-write.sh` | UBS-on-save hook |

## Edit (repo)

| Path | Change |
|------|--------|
| `AGENTS.md` | Remove beads steps from Landing the Plane; remove BEADS INTEGRATION block; remove UBS Quick Reference |
| `.github/copilot-instructions.md` | Remove Issue Tracking / `br` section |
| `CLAUDE.md` | Remove `br` / UBS bullets and “use br for tracking” rule |
| `.vscode/mcp.json` | Remove `beads` MCP server |

## Global uninstall (host)

| Artifact | Action |
|----------|--------|
| `~/.local/bin/ubs` | Delete |
| `~/.local/share/ubs/` | Delete (UBS modules) |
| `~/.local/bin/br` | Delete |
| `distill` | Not installed — no-op |
| `beads-mcp` | Not found on PATH — MCP entry removed only |

If `cargo uninstall br` / `ubs` applies, run that too.

## Success criteria

- Live agent paths no longer reference beads/`br`, UBS/`ubs`, or distill as required tooling.
- `.beads/` absent from the repo tree.
- `which ubs br distill` finds nothing on this machine.

## Authorization

User approved deletion of the listed paths and global binary removal on 2026-07-30 (“approved — delete those files. Remove also global binaries”).
