# Documentation

## Where things go

| Kind | Location | Naming |
|---|---|---|
| Design + implementation plans | `docs/plans/` | `YYYY-MM-DD-<topic>-design.md` and `YYYY-MM-DD-<topic>.md` |
| Durable analyses and investigations | `docs/reference/` | free-form |
| WebUI design source | `docs/design/` | Figma file, exported components, screenshots — not prose |
| Superseded conventions | `docs/archive/` | frozen — never add, never edit |
| User-facing documentation | `wiki/` | published to the GitHub Wiki |
| Agent standards loaded by `AGENTS.md` | `.serena/memories/` | see `AGENTS.md` |

`docs/plans/` is the path the superpowers `brainstorming` and `writing-plans` skills
already write to. Do not invent a new location; the tools will not follow you there.

A plan has an end date — it describes work that concludes. Reference does not — it stays
true after the work ships. Sort by that test.

## Plans

Newest first. A `-design.md` file is the approved shape; the matching plain file is the
task-by-task implementation plan. Some entries have only one of the two.

| Date | Topic | Design | Plan |
|---|---|---|---|
| 2026-08-13 | Camera firmware upgrade skill | ✅ | ✅ |
| 2026-08-12 | Firmware upgrade path (A/B) | ✅ | ✅ |
| 2026-08-01 | Docs consolidation | ✅ | ✅ |
| 2026-07-31 | Restart-resilience hardware fixes | ✅ | ✅ |
| 2026-07-31 | PR51 Copilot fixes | ✅ | ✅ |
| 2026-07-30 | UBS / beads / distill removal | ✅ | — |
| 2026-07-30 | SpecKit removal | ✅ | — |
| 2026-07-29 | Vendor-daemon restart resilience | ✅ | ✅ |
| 2026-07-29 | WebUI build improvements | ✅ `2026-07-29-webui-build-design.md` | ✅ `2026-07-29-webui-build-improvements.md` |
| 2026-07-28 | dashmap removal | — | ✅ |
| 2026-07-26 | Toolchain refresh | ✅ | ✅ |
| 2026-07-26 | Docker CI toolchain | ✅ | ✅ |
| 2026-07-26 | Dependency bump | ✅ | ✅ |
| 2026-07-25 | Async / hardware layer separation | — | ✅ |

The WebUI row is the one irregular pair: its design and plan stems differ. Left as-is
deliberately — renaming for symmetry is churn with no reader benefit.

## Reference

| Document | Subject |
|---|---|
| `docs/reference/architectural-complexity-analysis.md` | onvif-rust RTSP/video pipeline complexity and simplification roadmap |
| `docs/reference/rtp-send-latency-investigation.md` | Why RTP sends stall on the AK3918 |
| `docs/reference/video-flow.md` | Video path from sensor to client |
| `docs/reference/hack-process.md` | Reverse-engineering narrative for the camera |

## Design

`docs/design/` holds the WebUI design source, not documentation about it:

| Item | What |
|---|---|
| `docs/design/ONVIF.fig` | Figma source, authoritative for the Camera.UI theme |
| `docs/design/styles/globals.css` | Theme CSS, authoritative |
| `docs/design/components/`, `imports/`, `App.tsx` | Figma-exported React components, reference only — not the shipping WebUI, which lives in `cross-compile/www/` |
| `docs/design/img/` | Figma screenshots and mockups |
| `docs/design/prd.md`, `design_proposal.md`, `DESIGN_REVIEW.md` | Product requirements, design proposal, and design review for the web interface |
| `docs/design/export_figma_screenshots.py` | Regenerates `img/` |

Not exempted from CodeQL scanning — `.github/codeql/codeql-config.yml` excludes `cross-compile/anyka_reference/**` and a few other vendor paths, but not `docs/design/`. Most of this directory is Figma-exported reference code, not the shipping WebUI.

## Archive

See `docs/archive/README.md`.
