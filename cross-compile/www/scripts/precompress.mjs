/**
 * Pre-compress build output for tower-http's ServeDir precompressed_br/gzip.
 *
 * Replaces vite-plugin-compression, which silently emitted zero .br files.
 * Uses Node's built-in zlib: the `brotli` CLI is not installed on build hosts.
 *
 * Exits non-zero if it finds nothing to compress, so this can never again
 * succeed while doing nothing.
 */
import { readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
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

/** Compress every eligible asset under `root`, returning what it did. */
export function precompress(root) {
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

  return { compressed, rawTotal, brTotal, gzTotal };
}

const kb = (n) => `${(n / 1024).toFixed(1)} kB`;

/** Returns the process exit code rather than calling process.exit, so it is testable. */
export function main(argv = process.argv) {
  const root = argv[2];
  if (!root) {
    console.error('usage: precompress.mjs <dir>');
    return 2;
  }

  const { compressed, rawTotal, brTotal, gzTotal } = precompress(root);

  if (compressed === 0) {
    console.error(`precompress: no compressible assets found under ${root}`);
    return 1;
  }

  console.log(
    `precompress: ${compressed} files  raw ${kb(rawTotal)}  ` +
      `gzip ${kb(gzTotal)}  brotli ${kb(brTotal)}`,
  );
  return 0;
}

// Only when run as a CLI, not when imported by tests. Falls off the end on
// success rather than calling process.exit(0), which can truncate stdout.
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const code = main();
  if (code !== 0) process.exit(code);
}
