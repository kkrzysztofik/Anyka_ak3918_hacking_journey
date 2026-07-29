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
