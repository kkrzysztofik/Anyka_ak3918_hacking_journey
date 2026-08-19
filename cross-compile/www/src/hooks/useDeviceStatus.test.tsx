/**
 * useDeviceStatus hook tests
 */
import type { ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { getDiagnostics } from '@/services/diagnosticsService';
import type { Diagnostics } from '@/services/diagnosticsService';
import { getNetworkInterfaces } from '@/services/networkService';
import type { NetworkInterface } from '@/services/networkService';

import { STATUS_POLL_MS, useDeviceStatus } from './useDeviceStatus';

vi.mock('@/services/diagnosticsService', () => ({
  getDiagnostics: vi.fn(),
}));

vi.mock('@/services/networkService', () => ({
  getNetworkInterfaces: vi.fn(),
}));

const MOCK_DIAGNOSTICS: Diagnostics = {
  status: 'healthy',
  firmware_version: 'test',
  uptime: { process_s: 100, system_s: 7200 },
  cpu_percent: null,
  memory: null,
  storage: null,
  network: null,
  stream_frame_age_ms: null,
  components: [],
  degraded_services: [],
  vision: null,
  wifi: null,
};

function iface(hwAddress: string): NetworkInterface {
  return {
    token: 'wlan0',
    enabled: true,
    name: 'wlan0',
    hwAddress,
    linkSpeedMbps: 72,
    ipv4Enabled: true,
    dhcp: true,
    address: '192.168.2.198',
    prefixLength: 24,
    gateway: '',
  };
}

function makeWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return {
    queryClient,
    wrapper: ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    ),
  };
}

describe('useDeviceStatus', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getDiagnostics).mockResolvedValue(MOCK_DIAGNOSTICS);
    vi.mocked(getNetworkInterfaces).mockResolvedValue([iface('AA:AA:AA:AA:AA:AA')]);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('should refetch network interfaces on the status poll interval', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.mocked(getNetworkInterfaces)
      .mockResolvedValueOnce([iface('AA:AA:AA:AA:AA:AA')])
      .mockResolvedValue([iface('BB:BB:BB:BB:BB:BB')]);

    const { wrapper } = makeWrapper();
    const { result } = renderHook(() => useDeviceStatus(), { wrapper });

    await waitFor(() => {
      expect(result.current.primaryInterface?.hwAddress).toBe('AA:AA:AA:AA:AA:AA');
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(STATUS_POLL_MS);
    });

    await waitFor(() => {
      expect(result.current.primaryInterface?.hwAddress).toBe('BB:BB:BB:BB:BB:BB');
    });
  });
});
