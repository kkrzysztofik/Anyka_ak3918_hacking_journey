/**
 * Stream URL builder tests
 */
import { describe, expect, it } from 'vitest';

import { buildFlvUrl } from './streamUrl';

describe('buildFlvUrl', () => {
  // The point of these two: production must cross to port 8080, development
  // must not (the Vite proxy does it). Getting that backwards is the whole
  // reason this function exists.
  it('crosses to the FLV port in production', () => {
    expect(buildFlvUrl('sub', { isDev: false, hostname: '192.168.2.198' })).toBe(
      'http://192.168.2.198:8080/live/sub.flv',
    );
  });

  it('stays relative in development so the Vite proxy handles it', () => {
    expect(buildFlvUrl('main', { isDev: true, hostname: 'localhost' })).toBe('/live/main.flv');
  });
});
