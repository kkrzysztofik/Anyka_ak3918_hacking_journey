/**
 * Video codec label formatting tests
 */
import { describe, expect, it } from 'vitest';

import { formatAudioCodec, formatVideoCodec } from './videoCodec';

describe('formatAudioCodec', () => {
  it('names the AAC object type the camera emits', () => {
    expect(formatAudioCodec('mp4a.40.2')).toBe('AAC-LC');
  });

  it('keeps any other object type visible rather than hiding it', () => {
    expect(formatAudioCodec('mp4a.40.5')).toBe('AAC type 5');
    expect(formatAudioCodec('mp4a.40.99')).toBe('AAC type 99');
  });

  it('passes through non-AAC or malformed codec strings', () => {
    expect(formatAudioCodec('opus')).toBe('opus');
    expect(formatAudioCodec('mp4a.40')).toBe('mp4a.40');
  });

  it('returns undefined for an empty codec', () => {
    expect(formatAudioCodec(undefined)).toBeUndefined();
  });
});

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

  it('labels a level-11 Baseline tag with constraint_set3 as L1b, not L1.1', () => {
    // avc1.42100b: profile 66 (Baseline), constraint byte 0x10
    // (constraint_set3_flag), level_idc 11.
    expect(formatVideoCodec('avc1.42100b')).toBe('H.264 Baseline@L1b');
  });

  it('keeps L1.1 for level 11 without constraint_set3_flag', () => {
    expect(formatVideoCodec('avc1.42000b')).toBe('H.264 Baseline@L1.1');
  });

  it('passes through non-AVC or malformed codec strings', () => {
    expect(formatVideoCodec('hvc1.1.6.L93.B0')).toBe('hvc1.1.6.L93.B0');
    expect(formatVideoCodec('avc1')).toBe('avc1');
  });

  it('returns undefined for an empty codec', () => {
    expect(formatVideoCodec(undefined)).toBeUndefined();
  });
});
