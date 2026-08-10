# WebUI Cleanup — Design

**Date:** 2026-08-10
**Scope:** `cross-compile/www`
**Purpose:** Pre-work for the Diagnostics feature. Remove dead code and collapse
duplicate abstractions so the new page is built on one card family, one error
path, and one set of types.

## Origin

A repo-wide over-engineering audit of the frontend produced 23 findings. Two were
withdrawn after review:

- **DiagnosticsPage is not dead code.** Its mock scaffolding (`generateData`,
  the six literal log rows, the hardcoded device-info block) is superseded by the
  feature, not deleted by this cleanup. `Sparkline`, `card.tsx`, the three
  telemetry types, and the `/utilization` vite proxy all stay.
- **The dual TypeScript compilers are an active migration.** ESLint does not
  support TS 7, so `type-check` legitimately runs `tsc` and `tsc6` side by side.

One further finding was dropped during design (see Rejected below).

## Decisions

| Decision | Choice | Reason |
|---|---|---|
| Sequencing | Four PRs by risk | Independent revert; PR1 can land immediately |
| Unused CSS utilities | Split: keep telemetry-flavored, cut speculative | Diagnostics will use `.technical-panel`, `.skeleton*`, `.data-highlight` |
| Card merge | shadcn `Card` survives, `SettingsCard` dies | `AGENTS.md`: prefer shadcn primitives, don't invent base components |
| `StatusCard` | Keep, swap hardcoded hex for theme tokens | It is a composite, not a duplicate of `Card` |
| Credential storage | Memory-only; delete `crypto.ts` | Behavior is unchanged; the AES layer buys no security property |
| Fonts | Wire `index.css` into `main.tsx`, keep the deps | The bug is the missing import, not the dependency |

### Why the crypto layer goes

`crypto.ts` holds a non-exportable AES-GCM key in a module-level variable. The key
dies on page reload, so `decrypt` always fails after a refresh, the stored blob is
cleared, and the user re-authenticates. The `memoryAuth` branch already in
`useAuth` has identical observable semantics with no crypto at all. Deleting the
encrypted path changes nothing a user can see.

`security-guidelines.md` does not mandate client-side credential encryption; it
states that Basic auth credentials are base64-encoded, not encrypted, and should
be used only over HTTPS.

## Plan

### PR1 · `chore(webui): remove dead code` — ~-2190 lines, -4 deps

Nothing here rewrites live code. Every hunk removes something with zero
production callers.

| Cut | Lines |
|---|---|
| `components/users/UserDialogs.tsx` + test (no importers; Layout uses the other `ChangePasswordDialog`) | -897 |
| `test/schemaTestHelpers.ts`, `test/serviceTestHelpers.ts`, ~20 orphan helper exports | -400 |
| `config/cameraConfig.ts` + test (6 exports, 0 callers) | -348 |
| `components/SystemInfo.tsx` + test | -199 |
| `types/index.ts` prune — keep `SystemInfo`, `DataPoint`, `SystemUtilizationResponse` | -100 |
| `globals.css` speculative utilities + design-memory update | -90 |
| `soapBodies` dead keys, `escapeXmlAttribute`, unused `XMLBuilder` | -70 |
| Test-only exports: `checkDeviceReachable`, `getProfile`, `getVideoEncoderConfigurations`, `serializeEncrypted`, `deserializeEncrypted`, `setDeviceInformation` | -60 |
| `vite.config.ts` cruft: `react-router-dom` in `optimizeDeps`, unread `__APP_VERSION__`, constant-returning `chunkFileNames`, hand-parsed `assetFileNames`, stacked JSDoc | -20 |
| `tsconfig.json` redundant `paths` entries subsumed by `@/*` | -5 |
| Drop `dompurify`, `msw`, `eslint-plugin-tailwindcss`, `autoprefixer` | -4 deps |
| **Add** `import './index.css'` to `main.tsx` plus the Inter import | +2 |

CSS classes kept: `.technical-panel`, `.technical-data`, `.mono-value`,
`.badge-technical`, `.data-highlight`, `.value-transition`, `.empty-state*`,
`.skeleton*`. Cut: `.bg-noise`, `.bg-gradient-mesh`, `.technical-section`,
`.stagger-1..4`, `.btn-press`, `.ptz-button`, `.sidebar-icon`, `.focus-ring`,
`.accordion-*` keyframes, `.animate-spin-slow`.

`autoprefixer` goes because Tailwind v4's `@tailwindcss/postcss` prefixes via
Lightning CSS. `@fontsource/inter` stays — the design memory names it as the
fallback face, and it will finally load once `index.css` is imported.

`.card` becomes call-site-free once `SystemInfo` is deleted; cut it in the same
PR and update the design memory.

### PR2 · `refactor(webui): dedup` — ~-420 lines, zero visual change

- **#12** `React.forwardRef` → ref-as-prop across 15 ui files; replace the 29
  `data-testid={props['data-testid' as keyof typeof props] || 'x'}` casts with a
  destructured default.
- **#13** Adopt the existing `handleMutationError` at the 25 hand-rolled
  `onError` toast sites across 7 pages.
- **#8** LiveViewPage's 8 D-pad buttons → one `PtzButton` over a `DIRECTIONS` map.
- **#15** `profileService`: collapse the duplicated encoder-config mapping and
  merge `parseH264Options` / `parseJpegOptions`.
- **#17** Layout: dedup `NavLinkItem`'s two branches, hoist the repeated
  `path.replaceAll('/','-')`, drop the `useMemo` over a module constant, fix the
  `bg-dark-hover` no-op class.
- **#19** Give `soapRequest` a request-config parameter so `authService` can reuse
  `getDeviceInformation` instead of re-implementing its field mapping.

### PR-auth · `refactor(webui): drop client-side credential encryption` — ~-440 lines

Delete `utils/crypto.ts` and its test; remove the `storedData` / sessionStorage
path from `useAuth`, keeping the `memoryAuth` branch. Lands independently — it
touches no file the other PRs touch.

### PR3 · `refactor(webui): design system` — ~-415 lines, visible changes

- **#10** `SettingsCard` → shadcn `Card` across 7 pages; `p-5` becomes a size
  variant; 15 test testid refs rename `settings-card-*` → `card-*`.
- **#10b** `StatusCard`: replace `#1c1c1e` / `#3a3a3c` / `#2c2c2e` / `#6b6b6f`
  with theme tokens.
- **#6** Delete `ConnectionStatus`. `ConnectionStatusBadge` has one call site
  passing a literal `status="connected"`; the 3-state variant has none.
- **#9** Delete LiveViewPage's fake Stream URL, Stream Info, and Network Stats
  panels (all literals; the Copy button has no handler).

### Sequence

```
PR1 ──> PR2 ──> PR3 ──> Diagnostics
   └──> PR-auth (parallel, independent)
```

Diagnostics starts after PR3. If the feature needs to start sooner, pull #10
forward into PR1 and let the rest of PR3 land behind it.

## Verification

Shared gate on every PR, per `cross-compile/www/AGENTS.md`:

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
```

`type-check` runs both compilers. This matters for the `paths` change in PR1,
which is exactly the kind of edit that diverges between TS 6 and TS 7.

**PR1 — the signal is coverage going up.** Vitest's coverage config includes all
of `src/**`, so dead-but-tested files were inflating the denominator. A drop means
something live was deleted. Plus:

- Re-run the orphan-export scan after each deletion batch to catch cascades.
  Land it as `scripts/find-dead-exports.mjs` rather than keeping it a throwaway.
- `npm run analyze` before and after, to confirm the dropped deps actually left
  the bundle rather than surviving on a transitive edge.

**PR2 — the signal is that no test file changes.** These hunks are
behavior-preserving, so the suite passing *unmodified* is the proof. Known
exception: tests asserting on ref forwarding legitimately change for #12, and
those are called out individually rather than edited quietly.

**PR-auth — `useAuth.test.tsx` is rewritten,** so the check is manual: log in,
refresh, confirm the re-login prompt matches today's behavior.

**PR3 — tests change by design,** which removes them as an independent signal.
Verified on hardware: build, deploy to `.198`, load Identification and Network for
the `StatusCard` token swap and all 7 settings pages for the `Card` migration.
A deploy is verified with a real request, not a file listing.

Deliberate omission: no visual-regression tooling. Eyeballing 7 pages once is
cheaper than maintaining a baseline set. Revisit if design churn becomes routine.

## Risks

**Memory drift is a deliverable, not an afterthought.** `AGENTS.md` makes
`www-design-system.md` mandatory reading, so leaving it describing deleted CSS
utilities and a `SettingsCard` that no longer exists actively misleads the next
session. PR1 updates the `@layer components` list; PR3 updates the card pattern.
Same commit as the code, or it will not happen.

**Cascade orphans.** Deleting `UserDialogs` and `SystemInfo` orphans further
exports. Re-run the scan after each batch rather than assuming one pass suffices.

**PR1 is ~2200 lines to review.** All deletion, but still a lot to hold. One
commit per finding inside the PR so review can proceed finding-by-finding.

**Sequencing.** Diagnostics built before PR3 lands on `SettingsCard` and gets
rewritten.

## Rejected

**#18 — `fast-xml-parser` `isArray` option** (would have replaced 4 hand-rolled
`Array.isArray(x) ? x : x ? [x] : []` normalizations, -30 lines).

`safeString` returns its *default* when handed an array. A blanket `isArray` rule
that wraps a scalar some service reads directly turns every affected field into
`"Unknown"` silently — no crash, no type error, no lint warning, and the service
tests only catch it where a fixture happens to cover that tag. Making it safe
requires an explicit per-tag allowlist, which is more configuration than the 30
lines are worth. The hand-rolled normalizations stay.
