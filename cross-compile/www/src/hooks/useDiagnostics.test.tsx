/**
 * useDiagnostics hook tests
 *
 * Verifies snapshot forwarding, history accumulation, ring-buffer cap,
 * and the dataUpdatedAt guard that prevents duplicate history entries.
 */
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

import { getDiagnostics } from '@/services/diagnosticsService';
import type { Diagnostics } from '@/services/diagnosticsService';

import { useDiagnostics } from './useDiagnostics';

vi.mock('@/services/diagnosticsService', () => ({
  getDiagnostics: vi.fn(),
}));

const MOCK_DIAGNOSTICS: Diagnostics = {
  status: 'healthy',
  uptime: { process_s: 100, system_s: 200 },
  cpu_percent: 42,
  memory: { total_kb: 2048, used_kb: 1024 },
  storage: null,
  network: { rx_bps: 1000, tx_bps: 500 },
  stream_frame_age_ms: null,
  components: [],
  degraded_services: [],
  vision: null,
};

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

describe('useDiagnostics', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getDiagnostics).mockResolvedValue(MOCK_DIAGNOSTICS);
  });

  it('test_useDiagnostics_query_success_returns_snapshot', async () => {
    const { wrapper } = makeWrapper();
    const { result } = renderHook(() => useDiagnostics(), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(MOCK_DIAGNOSTICS);
  });

  it('test_useDiagnostics_new_data_appends_single_history_point', async () => {
    const { wrapper } = makeWrapper();
    const { result } = renderHook(() => useDiagnostics(), { wrapper });

    await waitFor(() => expect(result.current.history).toHaveLength(1));

    const point = result.current.history[0];
    expect(point.cpu).toBe(42);
    expect(point.memPct).toBeCloseTo(50);
    expect(point.rx).toBe(1000);
    expect(point.tx).toBe(500);
    expect(typeof point.t).toBe('number');
  });

  it(
    'test_useDiagnostics_many_updates_caps_history_at_sixty_samples',
    async () => {
      const { queryClient, wrapper } = makeWrapper();
      const { result } = renderHook(() => useDiagnostics(), { wrapper });

      await waitFor(() => expect(result.current.history).toHaveLength(1));

      for (let i = 0; i < 69; i++) {
        await act(async () => {
          await new Promise((resolve) => setTimeout(resolve, 2));
          await queryClient.invalidateQueries({ queryKey: ['diagnostics'] });
        });
      }

      await waitFor(() => expect(result.current.history).toHaveLength(60), { timeout: 10_000 });
    },
    20_000,
  );

  it('test_useDiagnostics_rerender_without_data_update_does_not_duplicate_history', async () => {
    const { wrapper } = makeWrapper();
    const { result, rerender } = renderHook(() => useDiagnostics(), { wrapper });

    await waitFor(() => expect(result.current.history).toHaveLength(1));
    const firstTimestamp = result.current.history[0].t;

    // Force a re-render that does NOT change the cached data.
    act(() => {
      rerender();
    });

    // History must stay at exactly 1 entry with the same timestamp.
    expect(result.current.history).toHaveLength(1);
    expect(result.current.history[0].t).toBe(firstTimestamp);
  });
});
