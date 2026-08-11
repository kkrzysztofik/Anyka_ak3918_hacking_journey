import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { findDeadExports, readSourceFiles } from './find-dead-exports.mjs';

describe('findDeadExports', () => {
  it('reports an export no other file references', () => {
    const files = new Map([
      ['./a.ts', 'export function used() {}\nexport function orphan() {}'],
      ['./b.ts', "import { used } from './a';\nused();"],
    ]);
    expect(findDeadExports(files)).toEqual([{ file: './a.ts', symbol: 'orphan' }]);
  });

  it('ignores references from the defining file itself', () => {
    const files = new Map([['./a.ts', 'export function solo() {}\nsolo();']]);
    expect(findDeadExports(files)).toEqual([{ file: './a.ts', symbol: 'solo' }]);
  });

  it('treats a type-only export as referenced when another file imports it', () => {
    const files = new Map([
      ['./t.ts', 'export interface Shape { x: number }'],
      ['./u.ts', "import type { Shape } from './t';"],
    ]);
    expect(findDeadExports(files)).toEqual([]);
  });

  it('does not report a $ identifier that another file references', () => {
    const files = new Map([
      ['./a.ts', 'export const $shape = 1;'],
      ['./b.ts', "import { $shape } from './a';\nconsole.log($shape);"],
    ]);
    expect(findDeadExports(files)).toEqual([]);
  });
});

describe('readSourceFiles', () => {
  it('excludes both .test and .spec files in prod-only mode', () => {
    const dir = mkdtempSync(join(tmpdir(), 'fde-'));
    try {
      writeFileSync(join(dir, 'a.ts'), 'export function a() {}');
      writeFileSync(join(dir, 'a.test.ts'), 'import { a } from "./a";');
      writeFileSync(join(dir, 'a.spec.ts'), 'import { a } from "./a";');
      const files = readSourceFiles(dir, { includeTests: false });
      expect([...files.keys()]).toEqual(['a.ts']);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
