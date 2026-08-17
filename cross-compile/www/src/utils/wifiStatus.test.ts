import { describe, expect, it } from 'vitest';

import { formatWifiChannel, formatWifiSecurity } from '@/utils/wifiStatus';

describe('wifiStatus', () => {
  it('formats channel and security when connected', () => {
    const wifi = {
      interface: 'wlan0',
      connected: true,
      ssid: 'kmk',
      frequency_mhz: 2437,
      channel: 6,
      security: 'WPA2',
      signal_dbm: -52,
    };

    expect(formatWifiChannel(wifi)).toBe('6');
    expect(formatWifiSecurity(wifi)).toBe('WPA2');
  });

  it('falls back to frequency when channel is missing', () => {
    expect(
      formatWifiChannel({
        interface: 'wlan0',
        connected: true,
        ssid: 'kmk',
        frequency_mhz: 2437,
        channel: null,
        security: 'WPA2',
        signal_dbm: null,
      }),
    ).toBe('2437 MHz');
  });

  it('returns em dash when not connected', () => {
    expect(formatWifiChannel(undefined)).toBe('—');
    expect(formatWifiSecurity(undefined)).toBe('—');
  });
});
