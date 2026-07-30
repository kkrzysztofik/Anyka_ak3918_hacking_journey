# WebUI Build Improvement — Design

Date: 2026-07-29
Scope: `cross-compile/www` build pipeline and bundle payload
Status: approved, ready for implementation planning

## Problem

The WebUI build succeeds while doing less than it claims. Two steps fail
silently:

- `vite-plugin-compression` prints its success banner with an empty file list
  and writes **zero `.br` files**. `onvif-rust`'s static handler
  (`cross-compile/onvif-rust/src/onvif/server.rs:602`) calls
  `.precompressed_br()`, so the server looks for Brotli assets that never
  existed and falls back to gzip on every request.
- `esbuild: { target: 'es2024' }` is inert under Vite 8, which replaced esbuild
  with Rolldown/Oxc. The build warns about it on every run; the browser target
  is whatever Rolldown defaults to, not what the config declares.

Separately, the eager payload is roughly 238 kB gzipped, and the single largest
artifact on the SD card renders synthetic data.

## Measured baseline

Attribution by summing `sourcesContent` per `node_modules` package from a
throwaway sourcemap build. Sizes are raw source bytes unless marked gz.

| Chunk | gz | Dominant sources |
|---|---|---|
| `vendor` | 94.6 kB | react-dom 525 kB, react-router 339 kB, zod 230 kB, tailwind-merge 103 kB |
| `ui-vendor` | 67.1 kB | react-hook-form 127 kB, sonner 64 kB, radix-select 52 kB, lucide 45 kB, zod 35 kB |
| `utils-vendor` | 24.4 kB | fast-xml-parser 90 kB + 6 transitive packages |
| `http-vendor` | 17.0 kB | axios 145 kB |
| `index` | 12.5 kB | app source 93 kB |
| `css` | 13.0 kB | — |
| `query-vendor` | 9.2 kB | @tanstack/query-core 66 kB |
| **eager total** | **≈238 kB** | |
| `charts-vendor` (lazy) | 107.0 kB | recharts 748 kB, @reduxjs/toolkit 78 kB, immer 50 kB, es-toolkit 49 kB, d3-scale 31 kB |

Two findings came only from measuring:

1. **`recharts@3` pulls Redux.** `@reduxjs/toolkit`, `react-redux` and `immer`
   are transitive dependencies of the charting library — 165 kB of raw source
   for internal state management.
2. **`zod` is split against itself.** `getChunkName` routes bare `zod` to
   `vendor` and `@hookform/resolvers` to `ui-vendor`. The resolver imports
   `zod/v4/core`, a different specifier, so Rolldown cannot dedupe across the
   manual chunk boundary and both chunks carry zod source.

A third finding came from reading `DiagnosticsPage.tsx`: all three `AreaChart`
instances plot `Math.random()` output (`generateData`, lines 129–131). The
107 kB chunk animates mock data.

## Guiding principle

The root defect is silent no-ops, not size. Every phase either removes a step
that can fail quietly or replaces it with one that fails loudly.

## Phases

Each phase is a separate commit, gated on
`npm run type-check && npm run lint && npm run test`, a rebuild, a chunk-size
comparison, and a device smoke test at `192.168.2.198`.

### Phase 1 — config sweep

Touches `vite.config.ts`, `scripts/build_sd_contents.sh`, `package.json`. No
application source.

- Replace both `viteCompression` plugins with `scripts/precompress.mjs`
  using Node's built-in `zlib.brotliCompressSync` and `gzipSync`. The `brotli`
  CLI is not installed on the build host and Node's zlib needs no dependency.
  **The script exits non-zero when it finds zero compressible assets** — the
  silent failure mode becomes a build failure.
- Move `esbuild.target` to `build.target: 'es2024'`.
- Switch `minify: 'terser'` to `'oxc'` (the Vite 8 default; terser was an
  explicit opt-in here). `drop_console` and `drop_debugger` move to
  `build.rolldownOptions.output.minify.compress`. Drop the `terser`
  devDependency. Oxc does not support property mangling; this config does not
  use it.
- Route `zod` and `@hookform/resolvers` to the same chunk to remove the
  duplicate.
- Skip `npm ci` when the `package-lock.json` hash matches the stamp in
  `node_modules/.anyka-lock-hash`.
- Remove the deprecated `@types/dompurify` stub package.
- Run `type-check` and `lint` before `vite build` in the SD path. `vite build`
  does not typecheck, so type errors currently reach the SD card.
- Raise `chunkSizeWarningLimit` from 200 to 350 with a comment naming the
  reason. After Phase 3 the only chunk above 200 kB should be `vendor`.

Verification: `find www -name '*.br' | wc -l` returns non-zero.

### Phase 2 — measurement ratchet

Add `scripts/analyze_webui_bundle.mjs`: a throwaway sourcemap build whose
`sourcesContent` lengths are summed per package and printed as a per-chunk
table. No dependencies, read-only. Makes bundle regressions answerable without
re-deriving the analysis.

### Phase 3 — replace recharts with an SVG sparkline

The charts are mock for now and are intended to plot real telemetry later, so
they stay on screen; only the renderer changes.

Add a `<Sparkline data={points} />` component — roughly 40 lines producing an
SVG area path with a hover line. Mock data feeds it today; `/utilization`
feeds it later with no bundle change at swap time. `AreaChart` with
`CartesianGrid`, `Tooltip`, `XAxis` and `YAxis` is more chart than 30 synthetic
points justify.

Removes `charts-vendor` entirely: −107 kB gz, −375 kB raw from the SD card, and
Redux, immer and d3 leave the dependency tree.

Wiring `/utilization` is explicitly out of scope. The data-in component keeps
Phase 3 independent of that endpoint's contract.

### Phase 4 — axios to fetch

`axios` is imported in one file, `src/services/api.ts`, using `axios.create`
plus two interceptors. `AbortController` is already used for cancellation, so
that path is unchanged. Replace with a request wrapper of roughly 50 lines.

−17 kB gz. Abandon this phase if the interceptors implement digest
re-challenge; verify at implementation time.

### Phase 5 — zod to zod/mini

The zod surface is `string`, `object`, `boolean`, `number`, `literal`, `enum`,
`coerce` and `infer` across 10 files, all supported by `zod/mini`. Chaining
changes: `z.string().min(5)` becomes `z.string().check(z.minLength(5))`. There
are 29 `z.string` call sites.

−10–15 kB gz. This is the weakest phase by value-per-risk and is ordered last
so it is cheap to abandon.

## Deferred

`react-router@8` contributes 339 kB of raw source for `Routes`, `Route` and
`useNavigate`. A `wouter` swap would touch all six routing files, and the
router is load-bearing for auth redirects. Recorded, not proposed.

## Projected outcome

Eager payload drops from roughly 238 kB gz to roughly 205 kB gz across phases
1, 4 and 5, and the 107 kB lazy chart chunk disappears in Phase 3. The on-wire
gain is larger once Brotli assets actually ship.
