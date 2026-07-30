// @vitest-environment node
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { brotliDecompressSync, gunzipSync } from 'node:zlib';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { main, precompress } from './precompress.mjs';

/** Compressible content must exceed MIN_BYTES (1024) to be picked up. */
const BIG = 'a'.repeat(2048);

let root;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'precompress-test-'));
});

afterEach(() => {
  rmSync(root, { recursive: true, force: true });
  vi.restoreAllMocks();
});

describe('precompress', () => {
  it('writes .br and .gz that decompress back to the original bytes', () => {
    writeFileSync(join(root, 'app.js'), BIG);

    const stats = precompress(root);

    expect(stats.compressed).toBe(1);
    expect(brotliDecompressSync(readFileSync(join(root, 'app.js.br'))).toString()).toBe(BIG);
    expect(gunzipSync(readFileSync(join(root, 'app.js.gz'))).toString()).toBe(BIG);
  });

  it('recurses into subdirectories', () => {
    mkdirSync(join(root, 'js', 'nested'), { recursive: true });
    writeFileSync(join(root, 'js', 'nested', 'chunk.js'), BIG);

    expect(precompress(root).compressed).toBe(1);
  });

  it('skips files below the size threshold', () => {
    writeFileSync(join(root, 'tiny.js'), 'x'.repeat(100));

    expect(precompress(root).compressed).toBe(0);
  });

  it('skips extensions that are not worth compressing', () => {
    writeFileSync(join(root, 'logo.png'), BIG);
    writeFileSync(join(root, 'font.woff2'), BIG);

    expect(precompress(root).compressed).toBe(0);
  });

  it('reports totals that reflect real compression', () => {
    writeFileSync(join(root, 'a.css'), BIG);
    writeFileSync(join(root, 'b.html'), BIG);

    const stats = precompress(root);

    expect(stats.compressed).toBe(2);
    expect(stats.rawTotal).toBe(BIG.length * 2);
    // Highly repetitive input, so both encoders must beat the raw size.
    expect(stats.brTotal).toBeLessThan(stats.rawTotal);
    expect(stats.gzTotal).toBeLessThan(stats.rawTotal);
  });
});

describe('main', () => {
  it('exits 2 when no directory is given', () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(main(['node', 'precompress.mjs'])).toBe(2);
    expect(err).toHaveBeenCalledWith(expect.stringContaining('usage:'));
  });

  // The guard this whole script exists for: vite-plugin-compression used to
  // emit zero .br files and still succeed.
  it('exits 1 when it finds nothing to compress', () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(main(['node', 'precompress.mjs', root])).toBe(1);
    expect(err).toHaveBeenCalledWith(expect.stringContaining('no compressible assets found'));
  });

  it('exits 0 and reports a summary on success', () => {
    writeFileSync(join(root, 'app.js'), BIG);
    const log = vi.spyOn(console, 'log').mockImplementation(() => {});

    expect(main(['node', 'precompress.mjs', root])).toBe(0);
    expect(log).toHaveBeenCalledWith(expect.stringContaining('precompress: 1 files'));
  });
});
