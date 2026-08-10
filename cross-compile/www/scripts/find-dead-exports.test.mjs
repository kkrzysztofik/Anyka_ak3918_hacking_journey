import { describe, expect, it } from 'vitest';
import { findDeadExports } from './find-dead-exports.mjs';

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
});
