/**
 * Lib Utils Tests
 */
import { describe, expect, it } from 'vitest';

import { cn } from './utils';

describe('cn', () => {
  it('should merge class names', () => {
    expect(cn('foo', 'bar')).toBe('foo bar');
  });

  it('should handle conditional classes', () => {
    const shouldIncludeBar = false;
    expect(cn('foo', shouldIncludeBar && 'bar', 'baz')).toBe('foo baz');
  });

  it('should handle undefined and null', () => {
    expect(cn('foo', undefined, 'bar', null)).toBe('foo bar');
  });

  it('should merge Tailwind classes with proper precedence', () => {
    // tailwind-merge should handle conflicting classes
    expect(cn('px-2 py-1', 'px-4')).toContain('px-4');
  });

  it('should handle empty strings', () => {
    expect(cn('foo', '', 'bar')).toBe('foo bar');
  });

  it('should handle arrays', () => {
    expect(cn(['foo', 'bar'], 'baz')).toBe('foo bar baz');
  });

  it('should handle objects', () => {
    expect(cn({ foo: true, bar: false, baz: true })).toContain('foo');
    expect(cn({ foo: true, bar: false, baz: true })).toContain('baz');
    expect(cn({ foo: true, bar: false, baz: true })).not.toContain('bar');
  });

  it('should handle mixed inputs', () => {
    const result = cn('foo', ['bar', 'baz'], { qux: true }, 'quux');
    expect(result).toContain('foo');
    expect(result).toContain('bar');
    expect(result).toContain('baz');
    expect(result).toContain('qux');
    expect(result).toContain('quux');
  });

  it('should handle no arguments', () => {
    expect(cn()).toBe('');
  });

  it('should merge conflicting Tailwind utilities correctly', () => {
    // px-2 should be overridden by px-4
    const result = cn('px-2 py-1', 'px-4');
    expect(result).not.toContain('px-2');
    expect(result).toContain('px-4');
    expect(result).toContain('py-1');
  });
});
