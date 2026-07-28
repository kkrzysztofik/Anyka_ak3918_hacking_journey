# WebUI Build Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the WebUI build stop failing silently, then remove roughly 130 kB gzipped of dependency payload from a camera that serves its UI off an SD card.

**Architecture:** Five phases, each a separate commit. Phase 1 is config-only and touches no application source. Phase 2 adds a measurement script. Phases 3–5 replace dependencies (`recharts`, `axios`, `zod`) whose usage is far narrower than their bundle cost. Every dependency swap is protected by an existing test file, so each phase is verified by tests written before the change.

**Tech Stack:** Vite 8 (Rolldown + Oxc), React 19, TypeScript, Vitest + React Testing Library, Node 20+ built-in `zlib`.

**Design doc:** `docs/plans/2026-07-29-webui-build-design.md`

---

## Orientation for the implementing engineer

You have not seen this codebase. Read this section before Task 1.

**Where things live.** The WebUI is a standalone npm project at
`cross-compile/www`. Every `npm` command in this plan runs from that
directory. Its build output does **not** stay there — `vite.config.ts` sets
`build.outDir` to `../../SD_card_contents/anyka_hack/onvif/www`, which is the
directory copied onto the camera's SD card.

**What serves it.** A Rust binary, `onvif-rust`, serves that directory using
`tower_http::services::ServeDir` with `.precompressed_br()` and
`.precompressed_gzip()` enabled
(`cross-compile/onvif-rust/src/onvif/server.rs:600-605`). That means: when a
browser sends `Accept-Encoding: br`, the server looks for `index.html.br` on
disk and serves it with `Content-Encoding: br`. If the file is absent it
silently falls back. **This is the central bug of Phase 1** — the build has
never written a single `.br` file, so that capability has been dead.

**Why this matters more than usual.** The target is an Anyka AK3918 camera with
36 MB of RAM serving over LAN. Bytes and CPU both cost.

**Commands you will run constantly** (all from `cross-compile/www`):

```bash
npm run type-check    # tsc, twice (two TS versions) — slow, ~40s
npm run lint          # eslint, --max-warnings 0
npm run test          # vitest run, 59 test files
npm run build         # vite build → SD_card_contents/.../www
```

**The full-quality gate before any commit in this plan:**

```bash
npm run type-check && npm run lint && npm run test
```

**Do not** use the repo's vendored Rust toolchain for anything here. That is for
the Rust crates; this is a pure npm project.

**Skills to load.** For any task touching `src/**`, use
@anyka-webui-testing for test patterns and @camera-webui-components for
component conventions. Use @superpowers:test-driven-development for Phases 3–5.

**Baseline to beat.** Record this before you start; compare after each phase:

| Chunk | gz |
|---|---|
| `vendor` | 94.65 kB |
| `ui-vendor` | 67.09 kB |
| `utils-vendor` | 24.43 kB |
| `http-vendor` | 16.95 kB |
| `index` | 12.52 kB |
| `css` | 12.95 kB |
| `query-vendor` | 9.24 kB |
| `charts-vendor` (lazy) | 106.99 kB |

---

# Phase 1 — Config sweep

No application source changes. Files touched: `cross-compile/www/vite.config.ts`,
`cross-compile/www/package.json`, `scripts/build_sd_contents.sh`, and one new
script.

## Task 1: Replace the broken compression plugins

The two `viteCompression` plugin instances in `vite.config.ts:66-67` produce
gzip files but zero brotli files, and print a success banner either way. Replace
both with a script that fails loudly.

**Files:**
- Create: `cross-compile/www/scripts/precompress.mjs`
- Modify: `cross-compile/www/vite.config.ts` (remove import line 5, plugin lines 66-67)
- Modify: `cross-compile/www/package.json` (build script, remove dependency)

**Step 1: Confirm the bug before fixing it**

```bash
cd cross-compile/www
find ../../SD_card_contents/anyka_hack/onvif/www -name '*.br' | wc -l
```

Expected: `0`. If it prints anything else, stop — the premise of this task is
wrong and you should re-read the design doc.

**Step 2: Write the script**

Create `cross-compile/www/scripts/precompress.mjs`:

```javascript
/**
 * Pre-compress build output for tower-http's ServeDir precompressed_br/gzip.
 *
 * Replaces vite-plugin-compression, which silently emitted zero .br files.
 * Uses Node's built-in zlib: the `brotli` CLI is not installed on build hosts.
 *
 * Exits non-zero if it finds nothing to compress, so this can never again
 * succeed while doing nothing.
 */
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { brotliCompressSync, constants, gzipSync } from 'node:zlib';

const COMPRESSIBLE = /\.(js|css|html|svg|json|txt|xml)$/;
const MIN_BYTES = 1024; // below this, a separate file is not worth the inode

function* walk(dir) {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) yield* walk(path);
    else yield path;
  }
}

const root = process.argv[2];
if (!root) {
  console.error('usage: precompress.mjs <dir>');
  process.exit(2);
}

let compressed = 0;
let rawTotal = 0;
let brTotal = 0;
let gzTotal = 0;

for (const file of walk(root)) {
  if (!COMPRESSIBLE.test(file)) continue;
  const source = readFileSync(file);
  if (source.length < MIN_BYTES) continue;

  const br = brotliCompressSync(source, {
    params: {
      [constants.BROTLI_PARAM_QUALITY]: 11,
      [constants.BROTLI_PARAM_SIZE_HINT]: source.length,
    },
  });
  const gz = gzipSync(source, { level: 9 });

  writeFileSync(`${file}.br`, br);
  writeFileSync(`${file}.gz`, gz);

  compressed += 1;
  rawTotal += source.length;
  brTotal += br.length;
  gzTotal += gz.length;
}

if (compressed === 0) {
  console.error(`precompress: no compressible assets found under ${root}`);
  process.exit(1);
}

const kb = (n) => `${(n / 1024).toFixed(1)} kB`;
console.log(
  `precompress: ${compressed} files  raw ${kb(rawTotal)}  ` +
    `gzip ${kb(gzTotal)}  brotli ${kb(brTotal)}`,
);
```

**Step 3: Prove the failure path works**

This is the check that matters — the whole point is a step that cannot pass
while doing nothing.

```bash
cd cross-compile/www
mkdir -p /tmp/precompress-empty
node scripts/precompress.mjs /tmp/precompress-empty; echo "exit=$?"
```

Expected output:

```
precompress: no compressible assets found under /tmp/precompress-empty
exit=1
```

If it exits `0`, the guard is broken. Fix before continuing.

```bash
rmdir /tmp/precompress-empty
```

**Step 4: Remove the plugins from `vite.config.ts`**

Delete line 5:

```typescript
import viteCompression from 'vite-plugin-compression';
```

Replace lines 64-68 with:

```typescript
  plugins: [react()],
```

**Step 5: Wire the script into the build**

In `cross-compile/www/package.json`, change the `build` script:

```json
    "build": "vite build && node scripts/precompress.mjs ../../SD_card_contents/anyka_hack/onvif/www",
```

Remove `"vite-plugin-compression": "^0.5.1",` from `devDependencies`.

**Step 6: Rebuild and verify brotli now exists**

```bash
cd cross-compile/www
npm install
npm run build
find ../../SD_card_contents/anyka_hack/onvif/www -name '*.br' | wc -l
find ../../SD_card_contents/anyka_hack/onvif/www -name '*.gz' | wc -l
```

Expected: both counts non-zero and equal. The `precompress:` summary should show
brotli meaningfully smaller than gzip (expect roughly 15–20% under).

**Step 7: Confirm the server side needs no change**

```bash
cd /home/kmk/dev/anyka-dev
grep -n "precompressed_br" cross-compile/onvif-rust/src/onvif/server.rs
```

Expected: line 602. No change needed — this task makes existing server code
functional. There is an existing Rust test at `server.rs:1108` covering gzip
negotiation; note in the commit message that brotli now has real files to serve.

**Step 8: Quality gate and commit**

```bash
cd cross-compile/www
npm run type-check && npm run lint && npm run test
cd /home/kmk/dev/anyka-dev
git add cross-compile/www/scripts/precompress.mjs cross-compile/www/vite.config.ts cross-compile/www/package.json cross-compile/www/package-lock.json
git commit -m "fix(webui): emit brotli assets the server already asks for

vite-plugin-compression printed a success banner while writing zero .br
files. server.rs:602 calls precompressed_br(), so that capability has
been dead. Replaced with a zlib script that exits non-zero when it finds
nothing to compress."
```

---

## Task 2: Apply the browser target that was being ignored

`vite.config.ts:74-82` sets `esbuild.target`. Vite 8 uses Oxc, not esbuild, so
this is discarded — the build warns about it on every run.

**Files:**
- Modify: `cross-compile/www/vite.config.ts:74-82`, `:117`

**Step 1: Reproduce the warning**

```bash
cd cross-compile/www
npm run build 2>&1 | head -5
```

Expected to contain: `Both esbuild and oxc options were set. oxc options will
be used and esbuild options will be ignored.`

**Step 2: Move the target**

Delete the whole `esbuild: { ... }` block (lines 74-82). Add `target` as the
first key inside `build:`:

```typescript
  build: {
    // Modern browsers only: Chrome 117+, Firefox 119+, Safari 17.4+, Edge 117+.
    // Was previously set under `esbuild`, which Vite 8 ignores (Oxc replaced it).
    target: 'es2024',
    outDir: '../../SD_card_contents/anyka_hack/onvif/www',
```

The deleted block also contained a `mode === 'type-check'` branch setting
`logLevel`. Type checking runs through `npm run type-check` (which invokes
`tsc`, not Vite), so that branch was dead. Do not port it.

**Step 3: Verify the warning is gone**

```bash
npm run build 2>&1 | grep -c "esbuild options will be ignored"
```

Expected: `0`.

**Step 4: Quality gate and commit**

```bash
npm run type-check && npm run lint && npm run test
git add cross-compile/www/vite.config.ts
git commit -m "fix(webui): move browser target to build.target for Vite 8

esbuild.target has been silently discarded since the Rolldown/Oxc
migration. Also drops a dead mode==='type-check' branch; type checking
runs through tsc, not Vite."
```

---

## Task 3: Switch minification from Terser to Oxc

`minify: 'terser'` (line 145) is an explicit opt-in to the slowest available
minifier. Oxc is Vite 8's default.

**Files:**
- Modify: `cross-compile/www/vite.config.ts:145-151`
- Modify: `cross-compile/www/package.json` (remove `terser` devDependency)

**Step 1: Record the baseline you must not regress**

```bash
cd cross-compile/www
npm run build 2>&1 | grep "index-"
grep -c "console\." ../../SD_card_contents/anyka_hack/onvif/www/js/index-*.js
```

Note the `index-*.js` size, and confirm the console count is `0` —
`drop_console` currently works and must keep working.

**Step 2: Replace the minifier config**

Replace lines 145-151 with:

```typescript
    // Oxc is the Vite 8 default minifier; Terser was an explicit opt-in and is
    // markedly slower. Oxc does not support property mangling, which this
    // project never used.
    minify: 'oxc',
```

and add `minify` under the existing `rollupOptions.output` block, alongside
`manualChunks`:

```typescript
        minify: {
          compress: {
            dropConsole: true,
            dropDebugger: true,
          },
        },
```

> **Verify the option names.** The Vite 8 migration guide states these move to
> `build.rolldownOptions.output.minify.compress.drop*` but does not pin the
> casing. If the build errors on an unknown key, try `drop_console` /
> `drop_debugger`. Step 3 is what proves you got it right — do not skip it.

**Step 3: Prove `drop_console` still works**

```bash
npm run build
grep -c "console\." ../../SD_card_contents/anyka_hack/onvif/www/js/index-*.js
```

Expected: `0`. **If this is non-zero, the option name is wrong.** Console calls
leaking into production on an embedded device is a real regression, not
cosmetic. Do not commit until this prints `0`.

**Step 4: Remove Terser**

Remove `"terser": "^5.49.0",` from `devDependencies`, then:

```bash
npm install
npm run build
```

Expected: build succeeds; `index-*.js` within a few percent of Step 1's size.

**Step 5: Quality gate and commit**

```bash
npm run type-check && npm run lint && npm run test
git add cross-compile/www/vite.config.ts cross-compile/www/package.json cross-compile/www/package-lock.json
git commit -m "perf(webui): minify with Oxc instead of Terser

Verified drop_console still strips all console calls from the bundle."
```

---

## Task 4: Stop splitting zod across two chunks

`getChunkName` routes bare `zod` to `vendor` and `@hookform/resolvers` to
`ui-vendor`. The resolver imports `zod/v4/core` — a different specifier — so
Rolldown cannot dedupe across the manual chunk boundary. Both chunks carry zod
source (230 kB raw in `vendor`, 35 kB raw in `ui-vendor`).

**Files:**
- Modify: `cross-compile/www/vite.config.ts:20-36`

**Step 1: Confirm the duplication**

```bash
cd cross-compile/www
npx vite build --outDir /tmp/zod-check --sourcemap --emptyOutDir >/dev/null 2>&1
grep -l "node_modules/zod/" /tmp/zod-check/js/*.js.map | wc -l
```

Expected: `2`. That is the duplication.

**Step 2: Route zod and its resolver to the same chunk**

In `getChunkName`, add a zod rule **before** the `ui-vendor` rule — order
matters, because `@hookform` is matched by the existing `ui-vendor` branch:

```typescript
  // zod and its resolver must land together: @hookform/resolvers imports
  // `zod/v4/core` while app code imports `zod`, and Rolldown cannot dedupe
  // those across a manual chunk boundary.
  if (id.includes('/zod/') || id.includes('@hookform/resolvers')) return 'validation-vendor';
```

**Step 3: Verify the duplicate is gone**

```bash
npx vite build --outDir /tmp/zod-check --sourcemap --emptyOutDir >/dev/null 2>&1
grep -l "node_modules/zod/" /tmp/zod-check/js/*.js.map | wc -l
rm -rf /tmp/zod-check
```

Expected: `1`. If still `2`, your rule is being shadowed — check that it
precedes the `ui-vendor` branch.

**Step 4: Compare chunk sizes**

```bash
npm run build 2>&1 | grep -E "vendor|validation"
```

Expected: `vendor` and `ui-vendor` both shrink; a new `validation-vendor`
appears. The **sum** of the three should be smaller than the previous sum of
`vendor` + `ui-vendor`. Record the numbers for the commit message.

**Step 5: Quality gate and commit**

```bash
npm run type-check && npm run lint && npm run test
git add cross-compile/www/vite.config.ts
git commit -m "perf(webui): stop bundling zod into two chunks at once"
```

---

## Task 5: Remove the deprecated types stub and a stale comment

**Files:**
- Modify: `cross-compile/www/package.json`
- Modify: `cross-compile/www/src/services/api.ts:26-29`

**Step 1: Remove the stub package**

`npm install` warns: `@types/dompurify@3.2.0: This is a stub types definition.
dompurify provides its own type definitions`.

Remove `"@types/dompurify": "^3.2.0",` from `devDependencies`, then:

```bash
cd cross-compile/www
npm install
npm run type-check
```

Expected: passes. `dompurify` ships its own types. If it fails, the stub was
masking a real typing problem — report it rather than reinstating the stub.

**Step 2: Fix the now-false comment**

`src/services/api.ts:26-29` currently reads:

```typescript
 * Note on Brotli: While .br files are pre-compressed in build output,
 * Brotli support requires server-side configuration to serve .br files
 * with Content-Encoding: br header when Accept-Encoding: br is present.
```

Both halves are wrong as of Task 1: `.br` files were *not* being produced, and
the server *does* have the configuration. Replace with:

```typescript
 * Note on Brotli: build output is pre-compressed to .br and .gz by
 * scripts/precompress.mjs, and onvif-rust serves them via ServeDir's
 * precompressed_br()/precompressed_gzip(). Browsers negotiate via
 * Accept-Encoding; no client-side handling is needed here.
```

**Step 3: Quality gate and commit**

```bash
npm run type-check && npm run lint && npm run test
git add cross-compile/www/package.json cross-compile/www/package-lock.json cross-compile/www/src/services/api.ts
git commit -m "chore(webui): drop deprecated @types/dompurify stub, correct brotli comment"
```

---

## Task 6: Skip `npm ci` when the lockfile has not changed

`scripts/build_sd_contents.sh:153-161` reinstalls 492 packages on every SD
build, roughly 7 seconds each time.

**Files:**
- Modify: `scripts/build_sd_contents.sh:153-161`

**Step 1: Read the existing block and the available log helpers**

```bash
sed -n '143,163p' scripts/build_sd_contents.sh
grep -n 'log_info\|log_step\|log_warn\|log_error' scripts/build_sd_contents.sh | head
```

Use whichever log helper actually exists; `log_info` below is a guess.

**Step 2: Add the freshness check**

npm rewrites `node_modules/.package-lock.json` into a different format, so
comparing it to `package-lock.json` is unreliable. Store a hash instead. Replace
the `( cd "${WWW_DIR}" ... )` subshell with:

```bash
  (
    cd "${WWW_DIR}"
    lock_hash_file="node_modules/.anyka-lock-hash"
    current_hash="$(sha256sum package-lock.json 2>/dev/null | cut -d' ' -f1)"
    if [[ -d node_modules && -f "${lock_hash_file}" && "$(cat "${lock_hash_file}")" == "${current_hash}" ]]; then
      log_info "Dependencies unchanged, skipping npm ci"
    else
      if [[ -f package-lock.json ]]; then
        npm ci
      else
        npm install
      fi
      printf '%s' "${current_hash}" > "${lock_hash_file}"
    fi
    npm run build
  )
```

**Step 3: Verify both paths**

```bash
cd /home/kmk/dev/anyka-dev
rm -f cross-compile/www/node_modules/.anyka-lock-hash
./scripts/build_sd_contents.sh 2>&1 | grep -E "npm ci|skipping npm ci"
```

Expected first run: `npm ci` runs. Run again — expected: `Dependencies
unchanged, skipping npm ci`.

**Step 4: Verify the cache is content-based, not mtime-based**

```bash
touch cross-compile/www/package-lock.json
./scripts/build_sd_contents.sh 2>&1 | grep -E "npm ci|skipping"
```

Expected: still skips. Touching does not change content, so this is correct.

**Step 5: Commit**

```bash
git add scripts/build_sd_contents.sh
git commit -m "perf(build): skip npm ci when the lockfile hash is unchanged"
```

---

## Task 7: Gate the SD build on type-check and lint

`vite build` does not typecheck. Type errors currently reach the SD card.

**Files:**
- Modify: `scripts/build_sd_contents.sh` (the subshell from Task 6)

**Step 1: Check the script's error handling mode**

```bash
grep -n "set -e\|set -euo" scripts/build_sd_contents.sh
```

**Step 2: Add the gate**

Immediately before `npm run build` in the subshell. If `set -e` is present:

```bash
    npm run type-check
    npm run lint
    npm run build
```

If it is absent, be explicit:

```bash
    npm run type-check || { log_error "WebUI type-check failed"; exit 1; }
    npm run lint || { log_error "WebUI lint failed"; exit 1; }
```

**Step 3: Prove the gate actually blocks a bad build**

Introduce a deliberate type error, confirm the build stops, then revert:

```bash
cd cross-compile/www
echo 'const broken: number = "not a number";' >> src/lib/utils.ts
cd /home/kmk/dev/anyka-dev
./scripts/build_sd_contents.sh; echo "exit=$?"
cd cross-compile/www && git checkout src/lib/utils.ts
```

Expected: non-zero exit, type-check failure reported, and **no** new files
written to `SD_card_contents/anyka_hack/onvif/www`.

A gate you have not seen fail is not a gate. Do not skip this step.

**Step 4: Commit**

```bash
git add scripts/build_sd_contents.sh
git commit -m "build: gate SD WebUI build on type-check and lint

vite build never typechecks, so type errors could reach the SD card."
```

---

# Phase 2 — Measurement ratchet

## Task 8: Add the bundle analysis script

Without this, "what got bigger" requires re-deriving the whole analysis.

**Files:**
- Create: `cross-compile/www/scripts/analyze-bundle.mjs`
- Modify: `cross-compile/www/package.json` (add `analyze` script)

**Step 1: Write the script**

The script only reads sourcemaps. The build that produces them is driven by the
npm script in Step 2 — keeping process spawning out of Node keeps this file to
pure parsing.

```javascript
/**
 * Attribute each output chunk's bytes to the npm packages inside it.
 *
 * Reads sourcemaps from a directory and sums sourcesContent length per package.
 * Raw source bytes, not gzipped output — use it to compare runs and find what
 * grew, not to predict wire size.
 *
 * Usage: node scripts/analyze-bundle.mjs <dir-of-js-and-maps>
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2];
if (!dir) {
  console.error('usage: analyze-bundle.mjs <dir>');
  process.exit(2);
}

const maps = readdirSync(dir).filter((f) => f.endsWith('.map'));
if (maps.length === 0) {
  console.error(`analyze-bundle: no sourcemaps in ${dir} — was the build run with --sourcemap?`);
  process.exit(1);
}

for (const file of maps) {
  const map = JSON.parse(readFileSync(join(dir, file), 'utf8'));
  const totals = {};
  map.sources.forEach((source, i) => {
    const bytes = (map.sourcesContent?.[i] ?? '').length;
    const match = /node_modules\/((?:@[^/]+\/)?[^/]+)/.exec(source);
    const key = match ? match[1] : '(app src)';
    totals[key] = (totals[key] ?? 0) + bytes;
  });

  const rows = Object.entries(totals).sort((a, b) => b[1] - a[1]);
  const chunkTotal = rows.reduce((sum, [, n]) => sum + n, 0);
  if (chunkTotal < 40_000) continue;

  const name = file.replace(/-[^-]+\.js\.map$/, '');
  console.log(`\n== ${name} — ${(chunkTotal / 1024).toFixed(0)} kB raw ==`);
  for (const [pkg, bytes] of rows.slice(0, 8)) {
    console.log(`  ${(bytes / 1024).toFixed(0).padStart(6)} kB  ${pkg}`);
  }
}
```

**Step 2: Add the npm script**

In `package.json` scripts — note this builds to `/tmp`, never to the SD output
directory, so running it cannot disturb a deployed build:

```json
    "analyze": "vite build --outDir /tmp/webui-analyze --sourcemap --emptyOutDir >/dev/null && node scripts/analyze-bundle.mjs /tmp/webui-analyze/js && rm -rf /tmp/webui-analyze",
```

**Step 3: Run it**

```bash
cd cross-compile/www
npm run analyze
```

Expected: a per-chunk table. `vendor` should show `react-dom`, `react-router`
and `tailwind-merge`; after Task 4, `zod` should appear under
`validation-vendor` only.

**Step 4: Verify the empty-input guard**

```bash
mkdir -p /tmp/analyze-empty
node scripts/analyze-bundle.mjs /tmp/analyze-empty; echo "exit=$?"
rmdir /tmp/analyze-empty
```

Expected: the "no sourcemaps" message and `exit=1`.

**Step 5: Confirm no leftover temp dir, then commit**

```bash
ls /tmp | grep -c webui-analyze   # expect: 0
npm run lint
git add cross-compile/www/scripts/analyze-bundle.mjs cross-compile/www/package.json
git commit -m "chore(webui): add bundle size attribution script"
```

---

# Phase 3 — Replace recharts with an SVG sparkline

`recharts@3` pulls `@reduxjs/toolkit`, `react-redux`, `immer`, `es-toolkit`,
`decimal.js-light` and `d3-*` — 107 kB gz — to draw three area charts of
`Math.random()` data. The charts are mock for now but are wanted on screen, so
the renderer changes and the charts stay.

Read @superpowers:test-driven-development before this phase.

## Task 9: Build the Sparkline component, test first

**Files:**
- Create: `cross-compile/www/src/components/common/Sparkline.tsx`
- Create: `cross-compile/www/src/components/common/Sparkline.test.tsx`

**Step 1: Write the failing test**

`recharts` usage is: one series for CPU, one for memory, **two** for network
(`upload` + `download`). The component must handle multiple series, a gradient
fill, and a fixed value domain.

```tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Sparkline } from './Sparkline';

const points = [
  { time: 0, value: 10 },
  { time: 1, value: 50 },
  { time: 2, value: 30 },
];

describe('Sparkline', () => {
  it('renders one filled area path per series', () => {
    render(
      <Sparkline
        data={points}
        series={[{ key: 'value', label: 'CPU', color: '#ef4444', unit: '%' }]}
        domain={[0, 100]}
      />,
    );
    expect(screen.getAllByTestId('sparkline-area')).toHaveLength(1);
  });

  it('renders an area per series when given two series', () => {
    const net = [
      { time: 0, upload: 2, download: 4 },
      { time: 1, upload: 3, download: 5 },
    ];
    render(
      <Sparkline
        data={net}
        series={[
          { key: 'download', label: 'Download', color: '#3b82f6', unit: ' Mbps' },
          { key: 'upload', label: 'Upload', color: '#22c55e', unit: ' Mbps' },
        ]}
      />,
    );
    expect(screen.getAllByTestId('sparkline-area')).toHaveLength(2);
  });

  it('produces a well-formed path with no NaN coordinates', () => {
    render(
      <Sparkline
        data={points}
        series={[{ key: 'value', label: 'CPU', color: '#ef4444' }]}
        domain={[0, 100]}
      />,
    );
    const d = screen.getAllByTestId('sparkline-area')[0].getAttribute('d');
    expect(d).toBeTruthy();
    expect(d).not.toContain('NaN');
  });

  it('honours a fixed domain rather than auto-scaling to the data', () => {
    // Values 10..50 under domain [0,100] must not touch the top of the box.
    render(
      <Sparkline
        data={points}
        series={[{ key: 'value', label: 'CPU', color: '#ef4444' }]}
        domain={[0, 100]}
      />,
    );
    const d = screen.getAllByTestId('sparkline-area')[0].getAttribute('d') ?? '';
    const ys = [...d.matchAll(/[ML](?:[\d.]+),([\d.]+)/g)].map((m) => Number(m[1]));
    expect(Math.min(...ys)).toBeGreaterThan(0);
  });

  it('renders nothing rather than crashing on empty data', () => {
    render(<Sparkline data={[]} series={[{ key: 'value', label: 'CPU', color: '#ef4444' }]} />);
    expect(screen.queryAllByTestId('sparkline-area')).toHaveLength(0);
  });
});
```

**Step 2: Run it and watch it fail**

```bash
cd cross-compile/www
npx vitest run src/components/common/Sparkline.test.tsx
```

Expected: FAIL — `Failed to resolve import "./Sparkline"`.

**Step 3: Implement the component**

Follow @camera-webui-components conventions. The maths: map each point into a
`viewBox` coordinate, join with `L`, then close the path down to the baseline
for the fill.

```tsx
import { useId, useMemo } from 'react';

export interface SparklineSeries {
  key: string;
  label: string;
  color: string;
  unit?: string;
}

export interface SparklineProps {
  data: Array<Record<string, number>>;
  series: SparklineSeries[];
  /** Fixed value range. Omit to scale to the data. */
  domain?: [number, number];
  className?: string;
}

const VIEW_W = 300;
const VIEW_H = 100;

function buildPaths(
  data: Array<Record<string, number>>,
  seriesKey: string,
  min: number,
  max: number,
) {
  const span = max - min || 1;
  const step = data.length > 1 ? VIEW_W / (data.length - 1) : 0;
  const coords = data.map((point, i) => {
    const y = VIEW_H - ((point[seriesKey] - min) / span) * VIEW_H;
    return `${(i * step).toFixed(2)},${y.toFixed(2)}`;
  });
  const line = `M${coords.join('L')}`;
  return { line, area: `${line}L${VIEW_W},${VIEW_H}L0,${VIEW_H}Z` };
}

export function Sparkline({ data, series, domain, className }: Readonly<SparklineProps>) {
  const gradientId = useId();

  const bounds = useMemo<[number, number]>(() => {
    if (domain) return domain;
    const values = data.flatMap((point) => series.map((s) => point[s.key]));
    return values.length ? [Math.min(...values), Math.max(...values)] : [0, 1];
  }, [data, series, domain]);

  if (data.length === 0) return null;

  return (
    <svg
      viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
      preserveAspectRatio="none"
      className={className ?? 'h-full w-full'}
      role="img"
      aria-label={series.map((s) => s.label).join(', ')}
    >
      <defs>
        {series.map((s) => (
          <linearGradient key={s.key} id={`${gradientId}-${s.key}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={s.color} stopOpacity={0.3} />
            <stop offset="95%" stopColor={s.color} stopOpacity={0} />
          </linearGradient>
        ))}
      </defs>

      {[0.25, 0.5, 0.75].map((fraction) => (
        <line
          key={fraction}
          x1={0}
          x2={VIEW_W}
          y1={VIEW_H * fraction}
          y2={VIEW_H * fraction}
          stroke="#333"
          strokeDasharray="3 3"
          strokeWidth={0.5}
          vectorEffect="non-scaling-stroke"
        />
      ))}

      {series.map((s) => {
        const { line, area } = buildPaths(data, s.key, bounds[0], bounds[1]);
        return (
          <g key={s.key}>
            <path d={area} fill={`url(#${gradientId}-${s.key})`} data-testid="sparkline-area" />
            <path
              d={line}
              fill="none"
              stroke={s.color}
              strokeWidth={2}
              vectorEffect="non-scaling-stroke"
            />
          </g>
        );
      })}
    </svg>
  );
}
```

> `vectorEffect="non-scaling-stroke"` matters: `preserveAspectRatio="none"`
> stretches the viewBox to the container, which would otherwise distort stroke
> width. This is the kind of detail recharts handled for you.

**Step 4: Run the tests**

```bash
npx vitest run src/components/common/Sparkline.test.tsx
```

Expected: 5 passed.

**Step 5: Commit**

```bash
npm run type-check && npm run lint
git add cross-compile/www/src/components/common/Sparkline.tsx cross-compile/www/src/components/common/Sparkline.test.tsx
git commit -m "feat(webui): add dependency-free SVG Sparkline component"
```

---

## Task 10: Swap DiagnosticsPage onto Sparkline

**Files:**
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx` (imports 15-22; charts at 219-249, 291-319, 370-410)
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.test.tsx:12-20`, `:108`

**Step 1: Update the existing test's mock first**

`DiagnosticsPage.test.tsx:13` mocks `recharts` and asserts on
`data-testid="area-chart"`. That mock is about to describe a package the page no
longer imports.

Delete the `vi.mock('recharts', ...)` block entirely. Then change the assertion
at line 108 from:

```tsx
expect(screen.getAllByTestId('area-chart').length).toBeGreaterThan(0);
```

to:

```tsx
expect(screen.getAllByTestId('sparkline-area').length).toBeGreaterThan(0);
```

**Step 2: Run and watch it fail**

```bash
npx vitest run src/pages/DiagnosticsPage.test.tsx
```

Expected: FAIL — no elements with testid `sparkline-area`. This proves the test
actually exercises the chart rendering path.

**Step 3: Replace the three chart blocks**

Remove the `recharts` import block (lines 15-22). Add:

```tsx
import { Sparkline } from '@/components/common/Sparkline';
```

CPU chart — replace the contents of `<div className="h-[180px] w-full">` at
lines 219-249:

```tsx
            <div className="h-[180px] w-full">
              <Sparkline
                data={cpuData}
                series={[{ key: 'value', label: 'CPU', color: '#ef4444', unit: '%' }]}
                domain={[0, 100]}
              />
            </div>
```

Memory chart — same shape, `data={memoryData}`, `domain={[0, 100]}`, and the
colour taken from the existing `colorMemory` gradient. **Read that gradient's
`stopColor`; do not guess it.**

Network chart — two series, and **no** `domain`: the original `<YAxis hide />`
had no domain, so it auto-scaled.

```tsx
            <div className="h-[180px] w-full">
              <Sparkline
                data={networkData}
                series={[
                  { key: 'download', label: 'Download', color: '#3b82f6', unit: ' Mbps' },
                  { key: 'upload', label: 'Upload', color: '#22c55e', unit: ' Mbps' },
                ]}
              />
            </div>
```

**Step 4: Handle `CustomTooltip`**

`CustomTooltip` (lines 100-126) is exported and shaped around recharts' payload
API. Check what references it:

```bash
grep -rn "CustomTooltip" src
```

If `DiagnosticsPage.tsx` and its test are the only references, delete it and any
test covering it — a recharts-shaped tooltip with no recharts is dead code. If
another file imports it, **stop and report**; the design assumed it was local.

**Step 5: Run the tests**

```bash
npx vitest run src/pages/DiagnosticsPage.test.tsx
```

Expected: PASS.

**Step 6: Remove recharts and verify the chunk is gone**

```bash
npm uninstall recharts
npm run build 2>&1 | grep -c "charts-vendor"
```

Expected: `0` — the chunk no longer exists. Also remove the now-dead
`charts-vendor` rule from `getChunkName` in `vite.config.ts:24`.

**Step 7: Confirm Redux left the tree**

```bash
npm ls @reduxjs/toolkit react-redux immer
```

Expected: all absent. They were transitive dependencies of recharts only.

**Step 8: Full gate, visual check, commit**

```bash
npm run type-check && npm run lint && npm run test
npm run dev   # open Diagnostics, confirm three charts still render
```

Charts are not something tests fully verify. Look at them.

```bash
git add cross-compile/www/src/pages/DiagnosticsPage.tsx cross-compile/www/src/pages/DiagnosticsPage.test.tsx cross-compile/www/vite.config.ts cross-compile/www/package.json cross-compile/www/package-lock.json
git commit -m "perf(webui): drop recharts for an SVG sparkline

recharts@3 pulled @reduxjs/toolkit, react-redux, immer, es-toolkit and
d3-* — 107 kB gz — to draw three area charts of Math.random() data."
```

---

## Task 11: Raise the chunk size warning limit

Deferred from Phase 1 deliberately: raise the limit only once the chunks it was
complaining about are actually gone.

**Files:**
- Modify: `cross-compile/www/vite.config.ts:143`

**Step 1: See what still exceeds 200 kB**

```bash
npm run build 2>&1 | grep -A5 "larger than 200"
```

**Step 2: Set the limit just above the largest legitimate chunk**

```typescript
    // `vendor` (react-dom + react-router) is legitimately ~310 kB raw and is
    // not further splittable without hurting caching. Set above it so this
    // warning means something when it fires.
    chunkSizeWarningLimit: 350,
```

Use your measured `vendor` size rounded up. If it exceeds 350 kB, use the actual
number — do not silence a warning you have not justified.

**Step 3: Verify the log is clean**

```bash
npm run build 2>&1 | grep -c "larger than"
```

Expected: `0`.

**Step 4: Commit**

```bash
git add cross-compile/www/vite.config.ts
git commit -m "build(webui): raise chunk warning limit above the vendor chunk"
```

---

# Phase 4 — Replace axios with fetch

`axios` is 145 kB raw / 17 kB gz, imported in exactly one file.

## Task 12: Swap `src/services/api.ts` to fetch

**Files:**
- Modify: `cross-compile/www/src/services/api.ts`
- Modify: `cross-compile/www/src/test/utils.ts:4` (imports `AxiosResponse`)
- Check: `cross-compile/www/src/services/api.test.ts` (414 lines — your safety net)

**Step 1: Abort criteria — check before writing anything**

```bash
cd cross-compile/www
grep -rn "digest\|Digest\|WWW-Authenticate\|nonce" src/services src/hooks
```

The design flagged this. From the read of `api.ts:39-63`, the request
interceptor only injects a pre-computed `Authorization` header and the response
interceptor only handles 401 — that is Basic auth and is safe to reimplement.
**If this grep shows digest re-challenge handling in the request path, stop and
report rather than proceeding.**

**Step 2: Run the existing tests and record the baseline**

```bash
npx vitest run src/services/api.test.ts
```

Expected: PASS. Note the test count. This suite must still pass in substance —
if you find yourself rewriting assertions about *behaviour* (as opposed to
types), you are changing behaviour, which is out of scope.

**Step 3: Read the whole test file before touching the source**

```bash
sed -n '1,80p' src/services/api.test.ts
```

It will tell you which axios surface the rest of the app depends on
(`apiClient.post`, response `.data`, thrown error shape). Your wrapper must
match that surface, not an idealised one — this is a drop-in replacement, not an
API redesign.

**Step 4: Write the replacement**

Preserve: the 10 s timeout, the two default headers, the async auth-header
injection, and the 401 handling. Keep the exported names `apiClient`,
`setAuthHeaderGetter`, `ENDPOINTS`, `ServiceEndpoint` so no caller changes.

Sketch — adapt to whatever surface Step 3 revealed:

```typescript
const DEFAULT_HEADERS = {
  'Content-Type': 'application/soap+xml; charset=utf-8',
  Accept: 'application/soap+xml, application/xml, */*',
};

export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly data: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

async function request(url: string, body: string, signal?: AbortSignal) {
  const headers: Record<string, string> = { ...DEFAULT_HEADERS };
  if (getAuthHeader) {
    const authHeader = await getAuthHeader();
    if (authHeader) headers.Authorization = authHeader;
  }

  const timeout = AbortSignal.timeout(10_000);
  const response = await fetch(url, {
    method: 'POST',
    headers,
    body,
    signal: signal ? AbortSignal.any([signal, timeout]) : timeout,
  });

  const text = await response.text();

  if (response.status === 401) {
    sessionStorage.removeItem('onvif_camera_auth');
    globalThis.dispatchEvent(new CustomEvent('auth:unauthorized'));
  }
  if (!response.ok) {
    throw new ApiError(`Request failed with status ${response.status}`, response.status, text);
  }
  return { data: text, status: response.status };
}

export const apiClient = { post: request };
```

`AbortSignal.any` is what keeps a caller-supplied signal *and* the timeout both
live; dropping either would be a behaviour change.

**Step 5: Update the test helper**

`src/test/utils.ts:4` imports `type AxiosResponse`. Replace with the shape your
wrapper returns.

**Step 6: Run everything**

```bash
npm run type-check && npm run lint && npm run test
```

Expected: all 59 test files pass. Any failure *outside* `api.test.ts` means a
caller depended on axios behaviour you did not replicate — fix the wrapper, not
the caller's test.

**Step 7: Remove axios and verify**

```bash
npm uninstall axios
npm run build 2>&1 | grep -c "http-vendor"
```

Expected: `0`. Remove the `http-vendor` rule from `getChunkName`.

**Step 8: Device smoke test — required**

This phase changes the network layer. Tests use mocks; the camera does not.

```bash
npm run build
```

Deploy and load the UI against the camera at `192.168.2.198`. Log in, load a
settings page that issues SOAP calls, and confirm a 401 still redirects to
login. **Report the result explicitly — do not claim this phase is done without
it.**

**Step 9: Commit**

```bash
git add -A cross-compile/www
git commit -m "perf(webui): replace axios with fetch in the single client module"
```

---

# Phase 5 — zod to zod/mini

Lowest value-per-risk phase, ordered last so it is cheap to abandon. **If Phases
1–4 have taken longer than expected, skipping this is a legitimate call** — say
so rather than rushing it.

## Task 13: Migrate schemas to `zod/mini`

**Files:**
- Modify: the 10 files importing from `zod` (find them below)
- Modify: `cross-compile/www/src/lib/schemas/*.ts` and their existing tests

**Step 1: Find every call site**

```bash
cd cross-compile/www
grep -rln "from 'zod'" src
grep -rhon "z\.[a-zA-Z]*" src | sed 's/.*://' | sort | uniq -c | sort -rn
```

Expected surface: `z.string` (29), `z.infer` (10), `z.object` (7), `z.boolean`
(6), `z.number` (4), `z.literal` (4), `z.enum` (3), `z.coerce` (1). All are
supported by `zod/mini`.

**Step 2: Confirm the schema tests exist and pass**

```bash
npx vitest run src/lib/schemas/
```

`identification.test.ts` and `network.test.ts` exist. These are your safety net.
The migration is mechanical but wide, and validation that stops firing is a
security-relevant regression, not a cosmetic one.

**Step 3: Migrate one file, run its test, then continue**

The API difference is chaining — `zod/mini` replaces method chains with
`check()`:

```typescript
// before
z.string().min(1, 'Required').max(64)
// after
z.string().check(z.minLength(1, 'Required'), z.maxLength(64))
```

Do **one file at a time** and run its test after each. Do not batch — a
wide sed with one wrong replacement is unbisectable.

**Step 4: Verify `@hookform/resolvers` still resolves**

```bash
npm run test
npm run dev   # submit a form with an invalid value, confirm the error shows
```

The resolver imports `zod/v4/core`, which `zod/mini` also targets, so this
should work. Forms silently accepting invalid input is the failure mode to watch
for.

**Step 5: Measure**

```bash
npm run analyze | grep -A3 validation-vendor
npm run build 2>&1 | grep validation
```

**If the saving is under roughly 8 kB gz, revert this phase** — the churn across
10 files is not worth less than that.

**Step 6: Commit**

```bash
npm run type-check && npm run lint && npm run test
git add -A cross-compile/www
git commit -m "perf(webui): migrate schemas to zod/mini"
```

---

# Closing out

## Task 14: Verify the whole pipeline and record results

**Step 1: Clean full build from scratch**

```bash
cd cross-compile/www
rm -rf node_modules
cd /home/kmk/dev/anyka-dev
./scripts/build_sd_contents.sh
```

Expected: succeeds; `npm ci` runs (no cached hash); type-check and lint gates
pass; the `precompress:` summary line appears.

**Step 2: Confirm every Phase 1 fix is still in place**

```bash
W=SD_card_contents/anyka_hack/onvif/www
find $W -name '*.br' | wc -l          # expect: non-zero
grep -c "console\." $W/js/index-*.js  # expect: 0
```

**Step 3: Compare against the baseline table in Orientation**

```bash
cd cross-compile/www && npm run analyze
```

Record final gz sizes per chunk. Expected direction: `charts-vendor` and
`http-vendor` gone entirely; `vendor` + `ui-vendor` + `validation-vendor` summing
below the old `vendor` + `ui-vendor`.

**Step 4: Device verification**

Deploy to the camera at `192.168.2.198` and confirm in browser devtools that
asset responses carry `Content-Encoding: br`. That header is the single clearest
proof the central bug is fixed — a passing build is not.

**Step 5: Report honestly**

State measured before/after numbers, and name any phase you skipped or reverted
and why. Use @superpowers:verification-before-completion before claiming this
plan is done.

---

## Risks and abort criteria

| Phase | Watch for | If it happens |
|---|---|---|
| 1 / Task 3 | `grep -c "console\." index-*.js` non-zero | Oxc option name is wrong; try snake_case. Do not commit until it is `0`. |
| 1 / Task 6 | `log_info` may not exist in the script's helper library | Check with grep; use whichever log helper is defined. |
| 3 / Task 10 | `CustomTooltip` imported outside DiagnosticsPage | Stop, report — the design assumed it was local. |
| 4 | Digest auth anywhere in the request path | Abort Phase 4; a fetch wrapper will not replicate it. |
| 4 | Any test outside `api.test.ts` fails | A caller depended on axios behaviour; fix the wrapper, never the caller's test. |
| 5 | Saving under ~8 kB gz | Revert. The churn is not worth it. |

## Explicitly out of scope

- Wiring `/utilization` to feed real telemetry into the sparklines. The component
  takes data as a prop, so this stays a separate, later change.
- Replacing `react-router` (339 kB raw for `Routes`/`Route`/`useNavigate`). A
  `wouter` swap touches all six routing files and the router is load-bearing for
  auth redirects.
- Replacing `fast-xml-parser`. It is 90 kB plus six transitive packages, but
  ONVIF SOAP parsing genuinely needs it.
