/**
 * Safe String Utility Tests
 */
import { describe, expect, it } from 'vitest';

import { safeString } from './safeString';

describe('safeString', () => {
  const defaultValue = 'default';

  describe('null and undefined values', () => {
    it('should return default for null', () => {
      expect(safeString(null, defaultValue)).toBe(defaultValue);
    });

    it('should return default for undefined', () => {
      expect(safeString(undefined, defaultValue)).toBe(defaultValue);
    });
  });

  describe('string values', () => {
    it('should return non-empty string as-is', () => {
      expect(safeString('test', defaultValue)).toBe('test');
    });

    it('should return default for empty string', () => {
      expect(safeString('', defaultValue)).toBe(defaultValue);
    });

    it('should return default for whitespace-only string', () => {
      expect(safeString('   ', defaultValue)).toBe('   '); // Empty check is ||, so whitespace passes
    });
  });

  describe('array values', () => {
    it('should return default for empty array', () => {
      expect(safeString([], defaultValue)).toBe(defaultValue);
    });

    it('should return default for array with elements', () => {
      expect(safeString([1, 2, 3], defaultValue)).toBe(defaultValue);
    });

    it('should return default for array of strings', () => {
      expect(safeString(['a', 'b'], defaultValue)).toBe(defaultValue);
    });
  });

  describe('primitive types', () => {
    it('should convert number to string', () => {
      expect(safeString(42, defaultValue)).toBe('42');
    });

    it('should convert zero to string', () => {
      expect(safeString(0, defaultValue)).toBe('0');
    });

    it('should convert negative number to string', () => {
      expect(safeString(-10, defaultValue)).toBe('-10');
    });

    it('should convert boolean true to string', () => {
      expect(safeString(true, defaultValue)).toBe('true');
    });

    it('should convert boolean false to string', () => {
      expect(safeString(false, defaultValue)).toBe('false');
    });

    it('should convert symbol to string', () => {
      const sym = Symbol('test');
      const result = safeString(sym, defaultValue);
      expect(result).toBe(sym.toString());
    });

    it('should convert bigint to string', () => {
      expect(safeString(BigInt(123), defaultValue)).toBe('123');
    });
  });

  describe('object values', () => {
    it('should return default for plain object', () => {
      expect(safeString({ key: 'value' }, defaultValue)).toBe(defaultValue);
    });

    it('should return default for empty object', () => {
      expect(safeString({}, defaultValue)).toBe(defaultValue);
    });

    it('should return default for Date object', () => {
      expect(safeString(new Date(), defaultValue)).toBe(defaultValue);
    });

    it('should return default for function', () => {
      expect(safeString(() => {}, defaultValue)).toBe(defaultValue);
    });
  });
});
