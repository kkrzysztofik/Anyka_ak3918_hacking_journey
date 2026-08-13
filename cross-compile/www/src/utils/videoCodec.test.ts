/**
 * Video codec label formatting tests
 */
import { describe, expect, it } from 'vitest';

import { formatVideoCodec } from './videoCodec';

describe('formatVideoCodec', () => {
  it('labels a Main profile tag with profile and level', () => {
    expect(formatVideoCodec('avc1.4de028')).toBe('H.264 Main@L4.0');
  });

  it('labels a High profile tag', () => {
    expect(formatVideoCodec('avc1.640028')).toBe('H.264 High@L4.0');
  });

  it('labels a Baseline profile tag', () => {
    expect(formatVideoCodec('avc1.42e01e')).toBe('H.264 Baseline@L3.0');
  });

  it('falls back to a numeric profile for unknown profile idc', () => {
    expect(formatVideoCodec('avc1.9e0028')).toBe('H.264 Profile 158@L4.0');
  });

  it('passes through non-AVC or malformed codec strings', () => {
    expect(formatVideoCodec('hvc1.1.6.L93.B0')).toBe('hvc1.1.6.L93.B0');
    expect(formatVideoCodec('avc1')).toBe('avc1');
  });

  it('returns undefined for an empty codec', () => {
    expect(formatVideoCodec(undefined)).toBeUndefined();
  });
});
