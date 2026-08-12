/**
 * DiagnosticsPage Tests
 *
 * All tests mock useDiagnostics so the page renders without network calls.
 * getLogs is mocked via @/services/diagnosticsService.
 */
import { screen } from '@testing-library/react';
import { waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { UseDiagnosticsResult } from '@/hooks/useDiagnostics';
import { useDiagnostics } from '@/hooks/useDiagnostics';
import type { Diagnostics } from '@/services/diagnosticsService';
import { getLogs } from '@/services/diagnosticsService';
import { renderWithProviders } from '@/test/componentTestHelpers';

import DiagnosticsPage from './DiagnosticsPage';

vi.mock('@/hooks/useDiagnostics');
vi.mock('@/services/diagnosticsService');

const BASE_DIAG: Diagnostics = {
  status: 'healthy',
  firmware_version: 'v1.2.3',
  uptime: { process_s: 3600, system_s: 3600 }, // no restart gap
  cpu_percent: 45,
  memory: { total_kb: 36864, used_kb: 18432 }, // 18 MB / 36 MB
  storage: { total_kb: 1048576, used_kb: 524288 }, // 512 MB / 1024 MB
  network: { rx_bps: 1_000_000, tx_bps: 500_000 },
  stream_frame_age_ms: 100,
  components: [{ name: 'onvif', status: 'healthy', message: null }],
  degraded_services: [],
  vision: null,
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

  it('test_DiagnosticsPage_render_shows_title_and_description', () => {
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-description')).toBeInTheDocument();
  });

  describe('CPU stat card', () => {
    it('test_DiagnosticsPage_cpu_percent_present_renders_value', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-cpu-usage-value')).toHaveTextContent('45%');
    });

    it('test_DiagnosticsPage_null_cpu_percent_renders_em_dash', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ cpu_percent: null }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-cpu-usage-value')).toHaveTextContent('\u2014');
    });
  });

  describe('Memory stat card', () => {
    it('test_DiagnosticsPage_memory_present_renders_megabytes', () => {
      renderWithProviders(<DiagnosticsPage />);
      // 18432 KB → 18 MB
      expect(screen.getByTestId('diagnostics-stat-memory-value')).toHaveTextContent('18 MB');
    });

    it('test_DiagnosticsPage_memory_present_renders_used_total_subvalue', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-memory-subvalue')).toHaveTextContent(
        '18 MB / 36 MB',
      );
    });

    it('test_DiagnosticsPage_null_memory_renders_em_dash', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ memory: null }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-memory-value')).toHaveTextContent('\u2014');
    });
  });

  describe('Storage stat card', () => {
    it('test_DiagnosticsPage_storage_card_renders', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-storage')).toBeInTheDocument();
    });

    it('test_DiagnosticsPage_storage_present_renders_megabytes', () => {
      renderWithProviders(<DiagnosticsPage />);
      // 524288 KB → 512 MB
      expect(screen.getByTestId('diagnostics-stat-storage-value')).toHaveTextContent('512 MB');
    });

    it('test_DiagnosticsPage_null_storage_renders_em_dash', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ storage: null }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-storage-value')).toHaveTextContent('\u2014');
    });
  });

  describe('Temperature card', () => {
    it('test_DiagnosticsPage_temperature_card_not_rendered', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.queryByTestId('diagnostics-stat-temperature')).not.toBeInTheDocument();
    });
  });

  describe('System Status card', () => {
    it('test_DiagnosticsPage_healthy_status_renders_healthy_label', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-system-status-value')).toHaveTextContent(
        'Healthy',
      );
    });

    it('test_DiagnosticsPage_degraded_status_renders_raw_status_string', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ status: 'degraded' }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stat-system-status-value')).toHaveTextContent(
        'degraded',
      );
    });
  });

  describe('Restart detection', () => {
    it('test_DiagnosticsPage_equal_uptime_gap_hides_restart_note', () => {
      // process_s === system_s → no restart
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.queryByTestId('diagnostics-restart-note')).not.toBeInTheDocument();
    });

    it('test_DiagnosticsPage_large_uptime_gap_shows_restart_note', () => {
      // 7200 - 600 = 6600 s gap → > 300 s threshold; note uses process uptime
      vi.mocked(useDiagnostics).mockReturnValue(
        makeResult({ uptime: { system_s: 7200, process_s: 600 } }),
      );
      renderWithProviders(<DiagnosticsPage />);
      const note = screen.getByTestId('diagnostics-restart-note');
      expect(note).toBeInTheDocument();
      expect(note).toHaveTextContent(/Restarted 10m 0s ago/);
    });

    it('test_DiagnosticsPage_threshold_uptime_gap_hides_restart_note', () => {
      vi.mocked(useDiagnostics).mockReturnValue(
        makeResult({ uptime: { system_s: 3600, process_s: 3300 } }), // gap = 300, not > 300
      );
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.queryByTestId('diagnostics-restart-note')).not.toBeInTheDocument();
    });
  });

  describe('Uptime rows', () => {
    it('test_DiagnosticsPage_uptime_rows_render', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-uptime-process')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-uptime-system')).toBeInTheDocument();
    });

    it('test_DiagnosticsPage_process_uptime_renders_human_readable', () => {
      // process_s = 3600 → 1h 0m
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-uptime-process')).toHaveTextContent('1h 0m');
    });
  });

  describe('Log filter buttons', () => {
    it('test_DiagnosticsPage_log_filter_buttons_render', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-log-filter-all')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-info')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-warn')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-error')).toBeInTheDocument();
    });

    it('test_DiagnosticsPage_log_filter_buttons_respond_to_clicks', async () => {
      const user = userEvent.setup();
      renderWithProviders(<DiagnosticsPage />);

      await user.click(screen.getByTestId('diagnostics-log-filter-info'));
      expect(screen.getByTestId('diagnostics-log-filter-info')).toBeInTheDocument();

      await user.click(screen.getByTestId('diagnostics-log-filter-all'));
      expect(screen.getByTestId('diagnostics-log-filter-all')).toBeInTheDocument();
    });
  });

  describe('Chart empty state', () => {
    it('test_DiagnosticsPage_empty_history_shows_chart_empty_states', () => {
      renderWithProviders(<DiagnosticsPage />); // history = []
      expect(screen.getByTestId('diagnostics-cpu-chart-empty')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-memory-chart-empty')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-network-chart-empty')).toBeInTheDocument();
    });
  });

  describe('Export button', () => {
    it('test_DiagnosticsPage_export_button_renders', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-export-button')).toBeInTheDocument();
    });
  });

  describe('Charts from history', () => {
    it('test_DiagnosticsPage_cpu_history_renders_sparkline', () => {
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
      expect(screen.getByTestId('sparkline-legend-cpu')).toHaveTextContent('CPU');
      expect(screen.getByTestId('sparkline-legend-cpu-value')).toHaveTextContent('45%');
    });

    it('test_DiagnosticsPage_insufficient_cpu_history_shows_empty_state', () => {
      renderWithProviders(<DiagnosticsPage />); // history = []
      expect(screen.getByTestId('diagnostics-cpu-chart-empty')).toBeInTheDocument();
    });

    it('test_DiagnosticsPage_zero_memory_history_shows_empty_state', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-memory-chart-empty')).toBeInTheDocument();
    });

    it('test_DiagnosticsPage_memory_history_renders_sparkline', () => {
      const now = Date.now();
      vi.mocked(useDiagnostics).mockReturnValue(
        makeResult({}, [
          { t: now - 5000, cpu: null, memPct: 50, rx: null, tx: null },
          { t: now, cpu: null, memPct: 55, rx: null, tx: null },
        ]),
      );
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.queryByTestId('diagnostics-memory-chart-empty')).not.toBeInTheDocument();
      expect(screen.getByTestId('sparkline-legend-memPct')).toHaveTextContent('Memory');
      expect(screen.getByTestId('sparkline-legend-memPct-value')).toHaveTextContent('55%');
    });
  });

  describe('Stream Health card', () => {
    it('test_DiagnosticsPage_frame_age_present_renders_value', () => {
      renderWithProviders(<DiagnosticsPage />); // stream_frame_age_ms = 100
      expect(screen.getByTestId('diagnostics-frame-age')).toHaveTextContent('100 ms');
    });

    it('test_DiagnosticsPage_null_frame_age_renders_em_dash', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ stream_frame_age_ms: null }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-frame-age')).toHaveTextContent('\u2014');
    });

    it('test_DiagnosticsPage_stale_frame_age_shows_stalled_indicator', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ stream_frame_age_ms: 6000 }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-stream-stalled')).toBeInTheDocument();
    });

    it('test_DiagnosticsPage_fresh_frame_age_hides_stalled_indicator', () => {
      renderWithProviders(<DiagnosticsPage />); // frame_age_ms = 100
      expect(screen.queryByTestId('diagnostics-stream-stalled')).not.toBeInTheDocument();
    });

    it('test_DiagnosticsPage_components_present_renders_list', () => {
      renderWithProviders(<DiagnosticsPage />); // components = [{ name: 'onvif', status: 'healthy' }]
      expect(screen.getByTestId('diagnostics-components-list')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-component-onvif')).toBeInTheDocument();
    });

    it('test_DiagnosticsPage_empty_components_hides_list', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ components: [] }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.queryByTestId('diagnostics-components-list')).not.toBeInTheDocument();
    });
  });

  describe('Network rates in Device Information', () => {
    it('test_DiagnosticsPage_network_present_renders_download_upload', () => {
      renderWithProviders(<DiagnosticsPage />); // network: { rx_bps: 1_000_000, tx_bps: 500_000 }
      expect(screen.getByTestId('diagnostics-network-download')).toHaveTextContent('1000 kbps');
      expect(screen.getByTestId('diagnostics-network-upload')).toHaveTextContent('500 kbps');
    });

    it('test_DiagnosticsPage_null_network_renders_em_dash', () => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ network: null }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-network-download')).toHaveTextContent('\u2014');
      expect(screen.getByTestId('diagnostics-network-upload')).toHaveTextContent('\u2014');
    });

    it('test_DiagnosticsPage_network_rows_always_render', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-network-download-row')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-network-upload-row')).toBeInTheDocument();
    });
  });

  describe('Log panel', () => {
    it('test_DiagnosticsPage_log_panel_renders_controls', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-system-logs-title')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-source-select')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-all')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-info')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-warn')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-log-filter-error')).toBeInTheDocument();
    });

    it('test_DiagnosticsPage_log_lines_render_after_fetch', async () => {
      vi.mocked(getLogs).mockResolvedValue([
        '2026-08-11 INFO Service started',
        '2026-08-11 WARN High latency',
      ]);
      renderWithProviders(<DiagnosticsPage />);
      await waitFor(() => {
        expect(screen.getByTestId('diagnostics-log-lines')).toBeInTheDocument();
      });
      expect(screen.getByTestId('diagnostics-log-lines')).toHaveTextContent('Service started');
    });

    it('test_DiagnosticsPage_empty_log_source_shows_unavailable_message', async () => {
      vi.mocked(getLogs).mockResolvedValue([]);
      renderWithProviders(<DiagnosticsPage />);
      await waitFor(() => {
        expect(screen.getByTestId('diagnostics-log-unavailable')).toBeInTheDocument();
      });
    });

    it('test_DiagnosticsPage_level_filter_calls_getLogs_with_level', async () => {
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

    it('test_DiagnosticsPage_source_change_refetches_logs', async () => {
      const user = userEvent.setup();
      renderWithProviders(<DiagnosticsPage />);

      const trigger = screen.getByTestId('diagnostics-log-source-select');
      await user.click(trigger);

      const vendorOption = screen.getByTestId('diagnostics-log-source-option-vendor_daemon');
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

    it('test_DiagnosticsPage_export_click_downloads_log_lines', async () => {
      const createObjectURL = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:test');
      const revokeObjectURL = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
      const clickMock = vi.fn();

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
      await waitFor(() => expect(revokeObjectURL).toHaveBeenCalledWith('blob:test'));
    });
  });

  describe('Day / Night Vision card', () => {
    const FULL_VISION: NonNullable<Diagnostics['vision']> = {
      mode: 'night',
      ae_luma: 42,
      ain0: 123,
      ir_led: true,
      ircut_a: false,
      ircut_b: true,
      white_led: null,
      supported: { ir_led: true, ircut: true, white_led: true },
    };

    it('test_DiagnosticsPage_vision_card_title_renders', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-vision-title')).toBeInTheDocument();
    });

    it.each([
      ['diagnostics-vision-mode', 'night'],
      ['diagnostics-vision-ae-luma', '42'],
      ['diagnostics-vision-ain0', '123'],
      ['diagnostics-vision-ir-led', 'On'],
      ['diagnostics-vision-ircut-a', 'Off'],
      ['diagnostics-vision-ircut-b', 'On'],
      ['diagnostics-vision-white-led', '\u2014'],
    ])('test_DiagnosticsPage_vision_present_renders_%s', (testId, expected) => {
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ vision: FULL_VISION }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId(testId)).toHaveTextContent(expected);
    });

    it('test_DiagnosticsPage_unsupported_ir_led_shows_na', () => {
      const vision: NonNullable<Diagnostics['vision']> = {
        ...FULL_VISION,
        ir_led: true,
        supported: { ...FULL_VISION.supported, ir_led: false },
      };
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ vision }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-vision-ir-led')).toHaveTextContent('n/a');
    });

    it('test_DiagnosticsPage_unsupported_ircut_shows_na', () => {
      const vision: NonNullable<Diagnostics['vision']> = {
        ...FULL_VISION,
        supported: { ...FULL_VISION.supported, ircut: false },
      };
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ vision }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-vision-ircut-a')).toHaveTextContent('n/a');
      expect(screen.getByTestId('diagnostics-vision-ircut-b')).toHaveTextContent('n/a');
    });

    it('test_DiagnosticsPage_unsupported_white_led_shows_na', () => {
      const vision: NonNullable<Diagnostics['vision']> = {
        ...FULL_VISION,
        white_led: true,
        supported: { ...FULL_VISION.supported, white_led: false },
      };
      vi.mocked(useDiagnostics).mockReturnValue(makeResult({ vision }));
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-vision-white-led')).toHaveTextContent('n/a');
    });

    it('test_DiagnosticsPage_null_vision_mode_renders_em_dash', () => {
      renderWithProviders(<DiagnosticsPage />); // BASE_DIAG has vision: null
      expect(screen.getByTestId('diagnostics-vision-mode')).toHaveTextContent('\u2014');
    });

    it('test_DiagnosticsPage_null_vision_sensors_renders_em_dash', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-vision-ae-luma')).toHaveTextContent('\u2014');
      expect(screen.getByTestId('diagnostics-vision-ain0')).toHaveTextContent('\u2014');
    });

    it('test_DiagnosticsPage_null_vision_lamps_renders_em_dash', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-vision-ir-led')).toHaveTextContent('\u2014');
      expect(screen.getByTestId('diagnostics-vision-ircut-a')).toHaveTextContent('\u2014');
      expect(screen.getByTestId('diagnostics-vision-ircut-b')).toHaveTextContent('\u2014');
      expect(screen.getByTestId('diagnostics-vision-white-led')).toHaveTextContent('\u2014');
    });

    it('test_DiagnosticsPage_null_vision_mode_field_renders_em_dash', () => {
      vi.mocked(useDiagnostics).mockReturnValue(
        makeResult({ vision: { ...FULL_VISION, mode: null } }),
      );
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-vision-mode')).toHaveTextContent('\u2014');
    });
  });

  describe('Device information', () => {
    it('test_DiagnosticsPage_shows_firmware_version', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-firmware-version')).toHaveTextContent('v1.2.3');
    });
  });

  describe('Firmware update', () => {
    it('test_DiagnosticsPage_firmware_upgrade_button_renders', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-firmware-upgrade-button')).toBeInTheDocument();
    });

    it('test_DiagnosticsPage_upgrade_click_opens_firmware_upgrade_dialog', async () => {
      const user = userEvent.setup();
      renderWithProviders(<DiagnosticsPage />);

      await user.click(screen.getByTestId('diagnostics-firmware-upgrade-button'));

      expect(screen.getByTestId('firmware-upgrade-dialog')).toBeInTheDocument();
    });
  });
});
