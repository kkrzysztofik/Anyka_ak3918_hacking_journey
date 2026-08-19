import { describe, expect, it } from 'vitest';

import { formatDuration } from './formatDuration';

describe('formatDuration', () => {
  it('should format sub-minute uptime', () => {
    expect(formatDuration(45)).toBe('45s');
  });

  it('should format minutes and seconds', () => {
    expect(formatDuration(125)).toBe('2m 5s');
  });

  it('should format hours and minutes', () => {
    expect(formatDuration(3661)).toBe('1h 1m');
  });

  it('should format days', () => {
    expect(formatDuration(90061)).toBe('1d 1h 1m');
  });
});
