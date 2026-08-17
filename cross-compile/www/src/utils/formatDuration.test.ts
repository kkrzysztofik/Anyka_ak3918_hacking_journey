import { describe, expect, it } from 'vitest';

import { formatDuration } from './formatDuration';

describe('formatDuration', () => {
  it('formats sub-minute uptime', () => {
    expect(formatDuration(45)).toBe('45s');
  });

  it('formats minutes and seconds', () => {
    expect(formatDuration(125)).toBe('2m 5s');
  });

  it('formats hours and minutes', () => {
    expect(formatDuration(3661)).toBe('1h 1m');
  });

  it('formats days', () => {
    expect(formatDuration(90061)).toBe('1d 1h 1m');
  });
});
