# Documentation

## Where things go

| Kind | Location | Naming |
|---|---|---|
| Designs (durable) + in-flight implementation plans | `docs/plans/` | `YYYY-MM-DD-<topic>-design.md` and `YYYY-MM-DD-<topic>.md` |
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

A `-design.md` file is the approved shape and the **durable** document. The matching plain
file is the task-by-task implementation plan: a working artifact, committed while the work
is in flight and deleted in the commit that ships it, with anything the design got wrong
moved into that design's `## Implementation outcome` section. If the plan has no matching
design, its durable findings go in `docs/reference/` instead. A plan that was superseded, or
that can no longer be executed as written, retires the same way — say so in the design.

Recover a deleted plan (the deletion commit's parent still holds it). Set `p` to the plan
path; keep it quoted, an unquoted `<name>` placeholder is a redirection:

```bash
p=docs/plans/2026-08-12-firmware-upgrade-path.md
git show "$(git log --diff-filter=D --format=%H -1 -- "$p")^:$p"
```

`ls docs/plans/` is the index. A hand-maintained table used to live here; it drifted to 17
rows against 64 designs and was deleted rather than repaired.

## Reference

| Document | Subject |
|---|---|
| `docs/reference/architectural-complexity-analysis.md` | onvif-rust RTSP/video pipeline complexity and simplification roadmap |
| `docs/reference/rtp-send-latency-investigation.md` | Why RTP sends stall on the AK3918 |
| `docs/reference/video-flow.md` | Video path from sensor to client |
| `docs/reference/hack-process.md` | Reverse-engineering narrative for the camera |
| `docs/reference/juan-flash-dump.md` | Stock JUAN AK3918EV200 flash dump (`juan-flash-dump.bin`) vs. our firmware; V500 `ak_motor.ko` ABI |
| `docs/reference/wifi-bring-up-findings.md` | Vendor wifi script defects (W1/W3/W6) and the findings that read as success (F1–F5) |
| `docs/reference/camera-cutover-traps.md` | Misdiagnoses and traps from the `.127` / `.146` anyka-init cutover |

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
