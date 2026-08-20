// @vitest-environment node
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { attributeChunk, main, resolveAnalyzedDir } from './analyze-bundle.mjs';

let dir;

/** A chunk is only reported once its raw total clears 40 kB. */
const big = (n) => 'x'.repeat(n);

function writeMap(name, sources, sourcesContent) {
  writeFileSync(join(dir, name), JSON.stringify({ sources, sourcesContent }));
}

beforeEach(() => {
  // Keep fixtures under cwd so resolveAnalyzedDir accepts them.
  mkdirSync(join(process.cwd(), '.tmp-analyze-bundle'), { recursive: true });
  dir = mkdtempSync(join(process.cwd(), '.tmp-analyze-bundle', 'test-'));
});

afterEach(() => {
  rmSync(dir, { recursive: true, force: true });
  vi.restoreAllMocks();
});

describe('attributeChunk', () => {
  it('groups bytes by npm package name', () => {
    const rows = attributeChunk({
      sources: ['../node_modules/react/index.js', '../node_modules/react/jsx.js'],
      sourcesContent: ['12345', '123'],
    });

    expect(rows).toEqual([['react', 8]]);
  });

  it('keeps the scope on scoped packages', () => {
    const rows = attributeChunk({
      sources: ['../node_modules/@tanstack/query-core/index.js'],
      sourcesContent: ['1234'],
    });

    expect(rows).toEqual([['@tanstack/query-core', 4]]);
  });

  it('attributes non-node_modules sources to the app', () => {
    const rows = attributeChunk({
      sources: ['src/pages/LiveViewPage.tsx'],
      sourcesContent: ['1234567'],
    });

    expect(rows).toEqual([['(app src)', 7]]);
  });

  it('sorts largest package first', () => {
    const rows = attributeChunk({
      sources: ['../node_modules/small/a.js', '../node_modules/large/b.js'],
      sourcesContent: ['1', '1234567890'],
    });

    expect(rows.map(([pkg]) => pkg)).toEqual(['large', 'small']);
  });

  it('treats a missing sourcesContent entry as zero bytes', () => {
    const rows = attributeChunk({
      sources: ['../node_modules/react/index.js'],
      sourcesContent: undefined,
    });

    expect(rows).toEqual([['react', 0]]);
  });
});

describe('main', () => {
  it('exits 2 when no directory is given', () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(main(['node', 'analyze-bundle.mjs'])).toBe(2);
    expect(err).toHaveBeenCalledWith(expect.stringContaining('usage:'));
  });

  it('exits 2 when the path escapes cwd', () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(main(['node', 'analyze-bundle.mjs', '/tmp'])).toBe(2);
    expect(err).toHaveBeenCalledWith(expect.stringContaining('refused path'));
    expect(resolveAnalyzedDir('/tmp')).toBeNull();
  });

  it('exits 1 when the directory holds no sourcemaps', () => {
    writeFileSync(join(dir, 'app.js'), 'not a map');
    const err = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(main(['node', 'analyze-bundle.mjs', dir])).toBe(1);
    expect(err).toHaveBeenCalledWith(
      expect.stringContaining('was the build run with --sourcemap?'),
    );
  });

  it('reports a chunk over the threshold, stripping the content hash', () => {
    writeMap('vendor-A1b2C3d4.js.map', ['../node_modules/react/index.js'], [big(50_000)]);
    const log = vi.spyOn(console, 'log').mockImplementation(() => {});

    expect(main(['node', 'analyze-bundle.mjs', dir])).toBe(0);
    expect(log).toHaveBeenCalledWith(expect.stringContaining('== vendor — 49 kB raw =='));
    expect(log).toHaveBeenCalledWith(expect.stringContaining('react'));
  });

  it('stays silent about chunks under the threshold', () => {
    writeMap('tiny-A1b2C3d4.js.map', ['../node_modules/react/index.js'], [big(1000)]);
    const log = vi.spyOn(console, 'log').mockImplementation(() => {});

    expect(main(['node', 'analyze-bundle.mjs', dir])).toBe(0);
    expect(log).not.toHaveBeenCalled();
  });

  it('lists at most the eight largest packages', () => {
    const sources = [];
    const contents = [];
    for (let i = 0; i < 12; i++) {
      sources.push(`../node_modules/pkg${i}/index.js`);
      contents.push(big(5000));
    }
    writeMap('big-A1b2C3d4.js.map', sources, contents);
    const log = vi.spyOn(console, 'log').mockImplementation(() => {});

    expect(main(['node', 'analyze-bundle.mjs', dir])).toBe(0);
    // One header line plus eight package rows.
    expect(log).toHaveBeenCalledTimes(9);
  });
});
