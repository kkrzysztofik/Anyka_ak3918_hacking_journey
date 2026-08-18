/**
 * Diagnostics Service Tests
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError, authorizedFetch, authorizedXhrPut } from '@/services/api';

import { getDiagnostics, getLogs, uploadFirmware } from './diagnosticsService';

vi.mock('@/services/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/api')>();
  return {
    ...actual,
    authorizedFetch: vi.fn(),
    authorizedXhrPut: vi.fn(),
  };
});

const MOCK_DIAGNOSTICS = {
  status: 'healthy',
  firmware_version: 'v1.2.3',
  uptime: { process_s: 3600, system_s: 7200 },
  cpu_percent: 12.5,
  memory: { total_kb: 131072, used_kb: 45000 },
  storage: { total_kb: 2097152, used_kb: 512000 },
  network: { rx_bps: 1024, tx_bps: 512 },
  stream_frame_age_ms: 33,
  components: [{ name: 'onvif_server', status: 'ok', message: null }],
  degraded_services: [],
  vision: null,
};

const PTZ_BLOCK = {
  enabled: true,
  opened: false,
  init_error: 'open /dev/ak-motor0: errno 19',
  self_check: null,
  position: null,
  moving: false,
  last_step_pos: null,
  commands_completed: 0,
};

function makeResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('diagnosticsService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('getDiagnostics', () => {
    it('test_getDiagnostics_ok_response_returns_parsed_snapshot', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(MOCK_DIAGNOSTICS));

      const result = await getDiagnostics();

      expect(result.status).toBe('healthy');
      expect(result.uptime.process_s).toBe(3600);
      expect(result.cpu_percent).toBe(12.5);
      expect(result.memory?.used_kb).toBe(45000);
      expect(result.network?.rx_bps).toBe(1024);
      expect(result.stream_frame_age_ms).toBe(33);
      expect(result.components[0].name).toBe('onvif_server');
      expect(result.degraded_services).toHaveLength(0);
    });

    it('should accept a diagnostics payload carrying a ptz block', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(
        makeResponse({ ...MOCK_DIAGNOSTICS, ptz: PTZ_BLOCK }),
      );
      const result = await getDiagnostics();
      expect(result.ptz?.init_error).toBe('open /dev/ak-motor0: errno 19');
    });

    it('should keep the step_readback value from the ptz block', async () => {
      const payload = {
        ...MOCK_DIAGNOSTICS,
        ptz: { ...PTZ_BLOCK, step_readback: 'unsupported' },
      };
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(payload));
      const result = await getDiagnostics();
      expect(result.ptz?.step_readback).toBe('unsupported');
    });

    it('should accept a ptz block without step_readback from an older bundle', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(
        makeResponse({ ...MOCK_DIAGNOSTICS, ptz: PTZ_BLOCK }),
      );
      const result = await getDiagnostics();
      expect(result.ptz?.step_readback).toBeUndefined();
    });

    it('should reject a ptz block with an unknown step_readback value', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(
        makeResponse({ ...MOCK_DIAGNOSTICS, ptz: { ...PTZ_BLOCK, step_readback: 'maybe' } }),
      );
      await expect(getDiagnostics()).rejects.toThrow(/unexpected shape/);
    });

    it('should accept a wifi block that omits link_quality from an older bundle', async () => {
      const wifi = {
        interface: 'wlan0',
        connected: true,
        ssid: 'kmk',
        frequency_mhz: 2437,
        channel: 6,
        security: 'WPA2',
        signal_dbm: -52,
      };
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse({ ...MOCK_DIAGNOSTICS, wifi }));
      const result = await getDiagnostics();
      expect(result.wifi?.ssid).toBe('kmk');
      expect(result.wifi?.link_quality).toBeUndefined();
    });

    it('should accept a diagnostics payload with no ptz key', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(MOCK_DIAGNOSTICS));
      await expect(getDiagnostics()).resolves.toBeDefined();
    });

    it('test_getDiagnostics_request_sends_authorization_via_authorizedFetch', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(MOCK_DIAGNOSTICS));

      await getDiagnostics();

      expect(vi.mocked(authorizedFetch)).toHaveBeenCalledWith(
        '/api/diagnostics',
        expect.objectContaining({ method: 'GET' }),
      );
    });

    it('test_getDiagnostics_abort_signal_forwards_to_authorizedFetch', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(MOCK_DIAGNOSTICS));
      const controller = new AbortController();

      await getDiagnostics(controller.signal);

      const init = vi.mocked(authorizedFetch).mock.calls[0][1] as RequestInit;
      expect(init.signal).toBe(controller.signal);
    });

    it('test_getDiagnostics_non_ok_response_throws_api_error', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse({ error: 'internal error' }, 500));

      await expect(getDiagnostics()).rejects.toThrow(ApiError);
    });

    it('test_getDiagnostics_invalid_shape_throws_api_error', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse({ status: 'healthy' }));

      await expect(getDiagnostics()).rejects.toThrow(ApiError);
    });

    it('test_getDiagnostics_invalid_json_throws_api_error', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(
        new Response('not json', { status: 200, headers: { 'Content-Type': 'application/json' } }),
      );

      await expect(getDiagnostics()).rejects.toThrow(ApiError);
    });
  });

  describe('getLogs', () => {
    it('test_getLogs_with_level_encodes_source_and_level_query_params', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(['line1', 'line2']));

      await getLogs('onvif_rust', 'info');

      const url = vi.mocked(authorizedFetch).mock.calls[0][0] as string;
      expect(url).toContain('source=onvif_rust');
      expect(url).toContain('level=info');
    });

    it('test_getLogs_with_lines_includes_lines_query_param', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(['a']));

      await getLogs('vendor_daemon', undefined, 50);

      const url = vi.mocked(authorizedFetch).mock.calls[0][0] as string;
      expect(url).toContain('lines=50');
    });

    it('test_getLogs_404_response_returns_empty_array', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(new Response('Not Found', { status: 404 }));

      const result = await getLogs('anyka_init');

      expect(result).toEqual([]);
    });

    it('test_getLogs_ok_response_returns_parsed_log_lines', async () => {
      const lines = ['[INFO] started', '[WARN] low memory'];
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(lines));

      const result = await getLogs('onvif_rust');

      expect(result).toEqual(lines);
    });

    it('test_getLogs_non_404_error_throws_api_error', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(new Response('Server Error', { status: 500 }));

      await expect(getLogs('onvif_rust')).rejects.toThrow(ApiError);
    });

    it('test_getLogs_invalid_json_throws_api_error', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(
        new Response('not json', { status: 200, headers: { 'Content-Type': 'application/json' } }),
      );

      await expect(getLogs('onvif_rust')).rejects.toThrow(ApiError);
    });
  });

  describe('uploadFirmware', () => {
    const file = new File(['firmware-bytes'], 'upgrade.bin', { type: 'application/octet-stream' });

    it('test_uploadFirmware_202_resolves_via_authorizedXhrPut', async () => {
      vi.mocked(authorizedXhrPut).mockResolvedValue({ status: 202, bodyText: '' });

      await expect(uploadFirmware(file)).resolves.toBeUndefined();

      expect(vi.mocked(authorizedXhrPut)).toHaveBeenCalledWith('/api/update', file, undefined);
    });

    it('test_uploadFirmware_forwards_progress_and_signal_options', async () => {
      vi.mocked(authorizedXhrPut).mockResolvedValue({ status: 202, bodyText: '' });
      const onProgress = vi.fn();
      const controller = new AbortController();

      await uploadFirmware(file, { onProgress, signal: controller.signal });

      expect(vi.mocked(authorizedXhrPut)).toHaveBeenCalledWith('/api/update', file, {
        onProgress,
        signal: controller.signal,
      });
    });

    it('test_uploadFirmware_non_202_throws_api_error', async () => {
      vi.mocked(authorizedXhrPut).mockResolvedValue({
        status: 400,
        bodyText: 'bad bundle',
      });

      await expect(uploadFirmware(file)).rejects.toMatchObject({
        name: 'ApiError',
        status: 400,
        data: 'bad bundle',
        message: 'Firmware upload failed with status 400',
      });
    });
  });
});
