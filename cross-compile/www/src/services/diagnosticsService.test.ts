/**
 * Diagnostics Service Tests
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { getDiagnostics, getLogs } from './diagnosticsService';

vi.mock('@/services/api', () => ({
  authorizedFetch: vi.fn(),
}));

import { authorizedFetch } from '@/services/api';

const MOCK_DIAGNOSTICS = {
  status: 'healthy',
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
    it('should return a parsed diagnostics snapshot', async () => {
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

    it('should send the Authorization header via authorizedFetch', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(MOCK_DIAGNOSTICS));

      await getDiagnostics();

      expect(vi.mocked(authorizedFetch)).toHaveBeenCalledWith(
        '/api/diagnostics',
        expect.objectContaining({ method: 'GET' }),
      );
    });

    it('should forward the AbortSignal to authorizedFetch', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(MOCK_DIAGNOSTICS));
      const controller = new AbortController();

      await getDiagnostics(controller.signal);

      const init = vi.mocked(authorizedFetch).mock.calls[0][1] as RequestInit;
      expect(init.signal).toBe(controller.signal);
    });

    it('should throw when the response is not ok', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(
        makeResponse({ error: 'internal error' }, 500),
      );

      await expect(getDiagnostics()).rejects.toThrow();
    });
  });

  describe('getLogs', () => {
    it('should encode source and level as query parameters', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(
        makeResponse(['line1', 'line2']),
      );

      await getLogs('onvif_rust', 'info');

      const url = vi.mocked(authorizedFetch).mock.calls[0][0] as string;
      expect(url).toContain('source=onvif_rust');
      expect(url).toContain('level=info');
    });

    it('should include the lines parameter', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(['a']));

      await getLogs('vendor_daemon', undefined, 50);

      const url = vi.mocked(authorizedFetch).mock.calls[0][0] as string;
      expect(url).toContain('lines=50');
    });

    it('should return empty array on 404 (missing source is normal)', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(
        new Response('Not Found', { status: 404 }),
      );

      const result = await getLogs('anyka_init');

      expect(result).toEqual([]);
    });

    it('should return parsed log lines array', async () => {
      const lines = ['[INFO] started', '[WARN] low memory'];
      vi.mocked(authorizedFetch).mockResolvedValue(makeResponse(lines));

      const result = await getLogs('onvif_rust');

      expect(result).toEqual(lines);
    });

    it('should throw on non-404 errors', async () => {
      vi.mocked(authorizedFetch).mockResolvedValue(
        new Response('Server Error', { status: 500 }),
      );

      await expect(getLogs('onvif_rust')).rejects.toThrow();
    });
  });
});
