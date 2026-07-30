/**
 * Attribute each output chunk's bytes to the npm packages inside it.
 *
 * Reads sourcemaps from a directory and sums sourcesContent length per package.
 * Raw source bytes, not gzipped output — use it to compare runs and find what
 * grew, not to predict wire size.
 *
 * Usage: node scripts/analyze-bundle.mjs <dir-of-js-and-maps>
 */
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

/** Minimum raw size for a chunk to be worth reporting. */
const REPORT_THRESHOLD = 40_000;
const PACKAGE_RE = /node_modules\/((?:@[^/]+\/)?[^/]+)/;

/** Sum a sourcemap's `sourcesContent` bytes per npm package, largest first. */
export function attributeChunk(map) {
  const totals = {};
  map.sources.forEach((source, i) => {
    const bytes = (map.sourcesContent?.[i] ?? '').length;
    const match = source.match(PACKAGE_RE);
    const key = match ? match[1] : '(app src)';
    totals[key] = (totals[key] ?? 0) + bytes;
  });
  return Object.entries(totals).sort((a, b) => b[1] - a[1]);
}

/** Returns the process exit code rather than calling process.exit, so it is testable. */
export function main(argv = process.argv) {
  const dir = argv[2];
  if (!dir) {
    console.error('usage: analyze-bundle.mjs <dir>');
    return 2;
  }

  const maps = readdirSync(dir).filter((f) => f.endsWith('.map'));
  if (maps.length === 0) {
    console.error(`analyze-bundle: no sourcemaps in ${dir} — was the build run with --sourcemap?`);
    return 1;
  }

  for (const file of maps) {
    const map = JSON.parse(readFileSync(join(dir, file), 'utf8'));
    const rows = attributeChunk(map);
    const chunkTotal = rows.reduce((sum, [, n]) => sum + n, 0);
    if (chunkTotal < REPORT_THRESHOLD) continue;

    const name = file.replace(/-[^-]+\.js\.map$/, '');
    console.log(`\n== ${name} — ${(chunkTotal / 1024).toFixed(0)} kB raw ==`);
    for (const [pkg, bytes] of rows.slice(0, 8)) {
      console.log(`  ${(bytes / 1024).toFixed(0).padStart(6)} kB  ${pkg}`);
    }
  }
  return 0;
}

// Only when run as a CLI, not when imported by tests.
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const code = main();
  if (code !== 0) process.exit(code);
}
