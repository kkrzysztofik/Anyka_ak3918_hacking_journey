import { describe, expect, it } from 'vitest';

import { formatWifiChannel, formatWifiQuality, formatWifiSecurity } from '@/utils/wifiStatus';

describe('wifiStatus', () => {
  it('should format channel, quality, and security when connected', () => {
    const wifi = {
      interface: 'wlan0',
      connected: true,
      ssid: 'kmk',
      frequency_mhz: 2437,
      channel: 6,
      security: 'WPA2',
      signal_dbm: -52,
      link_quality: '66/70',
    };

    expect(formatWifiChannel(wifi)).toBe('6');
    expect(formatWifiQuality(wifi)).toBe('66/70');
    expect(formatWifiSecurity(wifi)).toBe('WPA2');
  });

  it('should fall back to frequency when channel is missing', () => {
    expect(
      formatWifiChannel({
        interface: 'wlan0',
        connected: true,
        ssid: 'kmk',
        frequency_mhz: 2437,
        channel: null,
        security: 'WPA2',
        signal_dbm: null,
        link_quality: null,
      }),
    ).toBe('2437 MHz');
  });

  it('should return em dash when not connected', () => {
    const disconnected = {
      interface: 'wlan0',
      connected: false,
      ssid: 'kmk',
      frequency_mhz: 2437,
      channel: 6,
      security: 'WPA2',
      signal_dbm: -52,
      link_quality: '66/70',
    };

    expect(formatWifiChannel(disconnected)).toBe('—');
    expect(formatWifiQuality(disconnected)).toBe('—');
    expect(formatWifiSecurity(disconnected)).toBe('—');
    expect(formatWifiChannel(undefined)).toBe('—');
  });
});
