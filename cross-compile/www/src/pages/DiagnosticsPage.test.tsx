/**
 * DiagnosticsPage Tests
 *
 * All tests mock useDiagnostics so the page renders without network calls.
 * The service mock keeps TypeScript happy since getLogs is imported in later tasks.
 */
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { waitFor } from '@testing-library/react';

import type { UseDiagnosticsResult } from '@/hooks/useDiagnostics';
import { useDiagnostics } from '@/hooks/useDiagnostics';
import type { Diagnostics } from '@/services/diagnosticsService';
import { getLogs } from '@/services/diagnosticsService';
import { renderWithProviders } from '@/test/componentTestHelpers';

import DiagnosticsPage from './DiagnosticsPage';

vi.mock('@/hooks/useDiagnostics');
vi.mock('@/services/diagnosticsService');

const BASE_DIAG: Diagnostics = {
  status: 'ok',
  uptime: { process_s: 3600, system_s: 3600 }, // no restart gap
  cpu_percent: 45,
  memory: { total_kb: 36864, used_kb: 18432 }, // 18 MB / 36 MB
  storage: { total_kb: 1048576, used_kb: 524288 }, // 512 MB / 1024 MB
  network: { rx_bps: 1_000_000, tx_bps: 500_000 },
  stream_frame_age_ms: 100,
  components: [{ name: 'onvif', status: 'ok', message: null }],
  degraded_services: [],
};

function makeResult(
  overrides: Partial<Diagnostics> = {},
  history: UseDiagnosticsResult['history'] = [],
): UseDiagnosticsResult {
  return {
    data: { ...BASE_DIAG, ...overrides },
    history,
    isLoading: false,
    isError: false,
    error: null,
    isFetching: false,
    isSuccess: true,
    isPending: false,
    isRefetching: false,
    dataUpdatedAt: Date.now(),
    errorUpdatedAt: 0,
    failureCount: 0,
    failureReason: null,
    fetchStatus: 'idle',
    isLoadingError: false,
    isRefetchError: false,
    isStale: false,
    isPlaceholderData: false,
    status: 'success',
    refetch: vi.fn(),
  } as unknown as UseDiagnosticsResult;
}

describe('DiagnosticsPage', () => {
  beforeEach(() => {
    vi.mocked(useDiagnostics).mockReturnValue(makeResult());
    vi.mocked(getLogs).mockResolvedValue([]);
  });

  it('should render page title and description', () => {
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-description')).toBeInTheDocument();
  });

  describe('CPU stat card', () => {
    it('should render real cpu percent', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-cpu-usage-value')).toHaveTextContent('45%');
    });

    it('should show em-dash when cpu_percent is null', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ cpu_percent: null }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-cpu-usage-value')).toHaveTextContent('\u2014');
    });
  });

  describe('Memory stat card', () => {
    it('should render memory in megabytes', () => {
      renderWithProviders(<DiagnosticsPage />);
      // 18432 KB → 18 MB
      expect(screen.getByTestId('diagnostics-stat-memory-value')).toHaveTextContent('18 MB');
    });

    it('should show MB / MB subvalue', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-memory-subvalue')).toHaveTextContent(
        '18 MB / 36 MB',
      );
    });

    it('should show em-dash when memory is null', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ memory: null }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-memory-value')).toHaveTextContent('\u2014');
    });
  });

  describe('Storage stat card', () => {
    it('should render storage card', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-storage')).toBeInTheDocument();
    });

    it('should display storage in MB', () => {
      renderWithProviders(<DiagnosticsPage />);
      // 524288 KB → 512 MB
      expect(screen.getByTestId('diagnostics-stat-storage-value')).toHaveTextContent('512 MB');
    });

    it('should show em-dash when storage is null', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ storage: null }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-storage-value')).toHaveTextContent('\u2014');
    });
  });

  describe('Temperature card', () => {
    it('should NOT render a temperature stat card', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.queryByTestId('diagnostics-stat-temperature')).not.toBeInTheDocument();
    });
  });

  describe('System Status card', () => {
    it('should show Healthy when status is ok', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-system-status-value')).toHaveTextContent(
        'Healthy',
      );
    });

    it('should show raw status string when not ok', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ status: 'degraded' }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-system-status-value')).toHaveTextContent(
        'degraded',
      );
    });
  });

  describe('Restart detection', () => {
    it('should NOT show restart note when gap is within threshold', () => {
      // process_s === system_s → no restart
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.queryByTestId('diagnostics-restart-note')).not.toBeInTheDocument();
    });

    it('should flag recent restart when system_s - process_s exceeds threshold', () => {
      // 7200 - 600 = 6600 s gap → > 300 s threshold
      vi.mocked(useDiagnostics).mockReturnValue(
        makeResult({ uptime: { system_s: 7200, process_s: 600 } }),
      );
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-restart-note')).toBeInTheDocument();
    });

    it('should NOT show restart note when gap equals threshold exactly', () => {
      vi.mocked(useDiagnostics).mockReturnValue(
        makeResult({ uptime: { system_s: 3600, process_s: 3300 } }), // gap = 300, not > 300
      );
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.queryByTestId('diagnostics-restart-note')).not.toBeInTheDocument();
    });
  });

  describe('Uptime rows', () => {
    it('should render process and system uptime', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-uptime-process')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-uptime-system')).toBeInTheDocument();
    });

    it('should format process uptime in human-readable form', () => {
      // process_s = 3600 → 1h 0m
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-uptime-process')).toHaveTextContent('1h 0m');
    });
  });

  describe('Log filter buttons', () => {
    it('should render all log filter buttons', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-log-filter-all')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-info')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-warn')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-error')).toBeInTheDocument();
    });

    it('should handle log filter button clicks', async () => {
      const user = userEvent.setup();
      renderWithProviders(<DiagnosticsPage />);

      await user.click(screen.getByTestId('diagnostics-log-filter-info'));
      expect(screen.getByTestId('diagnostics-log-filter-info')).toBeInTheDocument();

      await user.click(screen.getByTestId('diagnostics-log-filter-all'));
      expect(screen.getByTestId('diagnostics-log-filter-all')).toBeInTheDocument();
    });
  });

  describe('Chart empty state', () => {
    it('should show empty state when history has fewer than two samples', () => {
      renderWithProviders(<DiagnosticsPage />); // history = []
      expect(screen.getByTestId('diagnostics-cpu-chart-empty')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-memory-chart-empty')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-network-chart-empty')).toBeInTheDocument();
    });
  });

  describe('Export button', () => {
    it('should render export button', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-export-button')).toBeInTheDocument();
    });
  });

  describe('Charts from history', () => {
    it('should render CPU sparkline when history has two or more cpu samples', () => {
      const now = Date.now();
      vi.mocked(useDiagnostics).mockReturnValue(
        makeResult({}, [
          { t: now - 5000, cpu: 30, memPct: 50, rx: null, tx: null },
          { t: now, cpu: 45, memPct: 55, rx: null, tx: null },
        ]),
      );
      renderWithProviders(<DiagnosticsPage />);
      // Chart should be visible — empty-state placeholder must be absent
      expect(screen.queryByTestId('diagnostics-cpu-chart-empty')).not.toBeInTheDocument();
    });

    it('should show empty state when fewer than two CPU samples are available', () => {
      renderWithProviders(<DiagnosticsPage />); // history = []
      expect(screen.getByTestId('diagnostics-cpu-chart-empty')).toBeInTheDocument();
    });

    it('should show memory chart empty state with zero samples', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-memory-chart-empty')).toBeInTheDocument();
    });

    it('should render memory sparkline when history has two or more memPct samples', () => {
      const now = Date.now();
      vi.mocked(useDiagnostics).mockReturnValue(
        makeResult({}, [
          { t: now - 5000, cpu: null, memPct: 50, rx: null, tx: null },
          { t: now, cpu: null, memPct: 55, rx: null, tx: null },
        ]),
      );
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.queryByTestId('diagnostics-memory-chart-empty')).not.toBeInTheDocument();
    });
  });

  describe('Stream Health card', () => {
    it('should render frame age from data', () => {
      renderWithProviders(<DiagnosticsPage />); // stream_frame_age_ms = 100
      expect(screen.getByTestId('diagnostics-frame-age')).toHaveTextContent('100 ms');
    });

    it('should show em-dash for frame age when null', () => {
      vi.mocked(useDiagnostics).mockReturnValue(
        makeResult({ stream_frame_age_ms: null }),
      );
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-frame-age')).toHaveTextContent('\u2014');
    });

    it('should flag stalled stream when frame_age_ms exceeds 5000 ms', () => {
      vi.mocked(useDiagnostics).mockReturnValue(
        makeResult({ stream_frame_age_ms: 6000 }),
      );
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stream-stalled')).toBeInTheDocument();
    });

    it('should NOT flag stalled stream when frame_age_ms is within threshold', () => {
      renderWithProviders(<DiagnosticsPage />); // frame_age_ms = 100
      expect(screen.queryByTestId('diagnostics-stream-stalled')).not.toBeInTheDocument();
    });

    it('should render components list when components are present', () => {
      renderWithProviders(<DiagnosticsPage />); // components = [{ name: 'onvif', status: 'ok' }]
      expect(screen.getByTestId('diagnostics-components-list')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-component-onvif')).toBeInTheDocument();
    });

    it('should NOT render components list when components array is empty', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ components: [] }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.queryByTestId('diagnostics-components-list')).not.toBeInTheDocument();
    });
  });

  describe('Log panel — Task 14', () => {
    it('should render log panel with source selector and level buttons', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-system-logs-title')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-source-select')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-all')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-info')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-warn')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-error')).toBeInTheDocument();
    });

    it('should render fetched log lines', async () => {
      vi.mocked(getLogs).mockResolvedValue([
        '2026-08-11 INFO Service started',
        '2026-08-11 WARN High latency',
      ]);
      renderWithProviders(<DiagnosticsPage />);
      await waitFor(() => {
        expect(screen.getByTestId('diagnostics-log-lines')).toBeInTheDocument();
      });
      expect(screen.getByTestId('diagnostics-log-lines')).toHaveTextContent(
        'Service started',
      );
    });

    it('should show unavailable message when source returns empty array', async () => {
      vi.mocked(getLogs).mockResolvedValue([]);
      renderWithProviders(<DiagnosticsPage />);
      await waitFor(() => {
        expect(screen.getByTestId('diagnostics-log-unavailable')).toBeInTheDocument();
      });
    });

    it('should call getLogs with level when level filter is changed', async () => {
      const user = userEvent.setup();
      renderWithProviders(<DiagnosticsPage />);

      const errorBtn = screen.getByTestId('diagnostics-log-filter-error');
      await user.click(errorBtn);

      await waitFor(() => {
        expect(vi.mocked(getLogs)).toHaveBeenCalledWith(
          'onvif_rust',
          'error',
          200,
          expect.anything(),
        );
      });
    });

    it('should refetch logs when source is changed via Select', async () => {
      const user = userEvent.setup();
      renderWithProviders(<DiagnosticsPage />);

      const trigger = screen.getByTestId('diagnostics-log-source-select');
      await user.click(trigger);

      const vendorOption = screen.getByTestId(
        'diagnostics-log-source-option-vendor_daemon',
      );
      await user.click(vendorOption);

      await waitFor(() => {
        expect(vi.mocked(getLogs)).toHaveBeenCalledWith(
          'vendor_daemon',
          undefined,
          200,
          expect.anything(),
        );
      });
    });

    it('should download loaded log lines on export click', async () => {
      const createObjectURL = vi.fn(() => 'blob:test');
      const revokeObjectURL = vi.fn();
      const clickMock = vi.fn();
      URL.createObjectURL = createObjectURL;
      URL.revokeObjectURL = revokeObjectURL;

      const createElementOrig = document.createElement.bind(document);
      vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
        if (tag === 'a') {
          const el = createElementOrig('a') as HTMLAnchorElement;
          el.click = clickMock;
          return el;
        }
        return createElementOrig(tag);
      });

      vi.mocked(getLogs).mockResolvedValue(['line one', 'line two']);
      const user = userEvent.setup();
      renderWithProviders(<DiagnosticsPage />);

      const exportBtn = await screen.findByTestId('diagnostics-export-button');
      await waitFor(() => expect(exportBtn).not.toBeDisabled());
      await user.click(exportBtn);

      expect(createObjectURL).toHaveBeenCalledWith(expect.any(Blob));
      expect(clickMock).toHaveBeenCalled();
      expect(revokeObjectURL).toHaveBeenCalledWith('blob:test');

      vi.restoreAllMocks();
    });
  });
});


