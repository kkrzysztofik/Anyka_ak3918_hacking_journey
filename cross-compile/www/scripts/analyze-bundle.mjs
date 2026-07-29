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
