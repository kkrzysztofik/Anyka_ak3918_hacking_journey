import { describe, expect, it } from 'vitest';

import type { Diagnostics } from '@/services/diagnosticsService';
import type { NetworkInterface } from '@/services/networkService';

import {
  formatLinkSpeedMbps,
  pickPrimaryNetworkInterface,
  strictHealthStatus,
  systemUptimeLabel,
} from './identificationStatusCard';

const healthyDiagnostics: Diagnostics = {
  status: 'healthy',
  firmware_version: 'test',
  uptime: { process_s: 100, system_s: 5000 },
  cpu_percent: null,
  memory: null,
  storage: null,
  network: null,
  stream_frame_age_ms: null,
  components: [],
  degraded_services: [],
  vision: null,
};

describe('strictHealthStatus', () => {
  it('returns unknown while loading', () => {
    expect(
      strictHealthStatus(undefined, { isError: false, isLoading: true }),
    ).toEqual({ label: '—', tone: 'unknown' });
  });

  it('returns unreachable on fetch error', () => {
    expect(
      strictHealthStatus(undefined, { isError: true, isLoading: false }),
    ).toEqual({ label: 'Unreachable', tone: 'unreachable' });
  });

  it('returns healthy when diagnostics report healthy with no degraded services', () => {
    expect(
      strictHealthStatus(healthyDiagnostics, { isError: false, isLoading: false }),
    ).toEqual({ label: 'Healthy', tone: 'healthy' });
  });

  it('returns degraded when degraded_services is non-empty', () => {
    const diagnostics = {
      ...healthyDiagnostics,
      degraded_services: ['Stream Health'],
    };
    expect(strictHealthStatus(diagnostics, { isError: false, isLoading: false })).toEqual({
      label: 'Degraded',
      tone: 'degraded',
      detail: 'Stream Health',
    });
  });
});

describe('systemUptimeLabel', () => {
  it('formats system uptime from diagnostics', () => {
    expect(systemUptimeLabel(healthyDiagnostics)).toBe('1h 23m');
  });

  it('returns em dash when diagnostics missing', () => {
    expect(systemUptimeLabel(undefined)).toBe('—');
  });
});

describe('pickPrimaryNetworkInterface', () => {
  const eth: NetworkInterface = {
    token: 'eth0',
    enabled: true,
    name: 'eth0',
    hwAddress: '00:11:22:33:44:55',
    linkSpeedMbps: 100,
    ipv4Enabled: true,
    dhcp: true,
    address: '192.168.1.2',
    prefixLength: 24,
    gateway: '',
  };

  const wlan: NetworkInterface = {
    ...eth,
    token: 'wlan0',
    name: 'wlan0',
    hwAddress: 'C0:4B:24:DA:4D:EB',
    linkSpeedMbps: 72,
  };

  it('prefers wlan0 over eth0', () => {
    expect(pickPrimaryNetworkInterface([eth, wlan])).toEqual(wlan);
  });

  it('falls back to first enabled interface with a MAC', () => {
    expect(pickPrimaryNetworkInterface([{ ...eth, enabled: false }, wlan])).toEqual(wlan);
  });
});

describe('formatLinkSpeedMbps', () => {
  it('formats positive speeds', () => {
    expect(formatLinkSpeedMbps(100)).toBe('100 Mbps');
  });

  it('returns em dash for missing or zero speed', () => {
    expect(formatLinkSpeedMbps(null)).toBe('—');
    expect(formatLinkSpeedMbps(0)).toBe('—');
  });
});
