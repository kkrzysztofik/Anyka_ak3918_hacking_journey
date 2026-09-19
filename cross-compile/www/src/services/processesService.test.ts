/**
 * Processes Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError, authorizedFetch } from '@/services/api';
import { getProcesses, restartService } from '@/services/processesService';

vi.mock('@/services/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/api')>();
  return {
    ...actual,
    authorizedFetch: vi.fn(),
  };
});

const wellFormed = {
  supervised: [
    {
      name: 'onvif',
      state: 'running',
      pid: 42,
      uptime_s: 90,
      restarts: 3,
      retry_in_s: 0,
    },
    {
      name: 'vendor-daemon',
      state: 'backoff',
      pid: null,
      uptime_s: 0,
      restarts: 7,
      retry_in_s: 12,
    },
  ],
  processes: [
    { pid: 42, ppid: 1, comm: 'onvif-rust.bin', state: 'S', rss_kb: 8192, cpu_time_s: 3 },
    { pid: 1, ppid: 0, comm: 'init', state: 'S', rss_kb: 0, cpu_time_s: 0 },
  ],
};

describe('processesService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getProcesses', () => {
    it('should parse a well-formed response', async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(new Response(JSON.stringify(wellFormed), {
        status: 200,
      }));

      const data = await getProcesses();

      expect(authorizedFetch).toHaveBeenCalledWith(
        '/api/processes',
        expect.objectContaining({ method: 'GET' }),
      );
      expect(data.supervised).toHaveLength(2);
      expect(data.supervised?.[0]).toEqual({
        name: 'onvif',
        state: 'running',
        pid: 42,
        uptime_s: 90,
        restarts: 3,
        retry_in_s: 0,
      });
      expect(data.processes).toHaveLength(2);
      expect(data.processes[0].comm).toBe('onvif-rust.bin');
    });

    it('should treat supervised: null as valid (control socket unavailable)', async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(
        new Response(
          JSON.stringify({ supervised: null, processes: wellFormed.processes }),
          { status: 200 },
        ),
      );

      const data = await getProcesses();

      expect(data.supervised).toBeNull();
      expect(data.processes).toHaveLength(2);
    });

    it('should throw ApiError on a malformed supervised row', async () => {
      const bad = {
        ...wellFormed,
        supervised: [{ name: 'onvif', state: 'running', pid: 42 }], // missing fields
      };
      vi.mocked(authorizedFetch).mockResolvedValueOnce(new Response(JSON.stringify(bad), {
        status: 200,
      }));

      await expect(getProcesses()).rejects.toThrow(ApiError);
    });

    it('should throw ApiError on a 503', async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(new Response('nope', { status: 503 }));

      await expect(getProcesses()).rejects.toThrow(ApiError);
    });
  });

  describe('restartService', () => {
    it('should resolve on 202 and POST to the service path', async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(new Response('', { status: 202 }));

      await expect(restartService('onvif')).resolves.toBeUndefined();

      expect(authorizedFetch).toHaveBeenCalledWith(
        '/api/services/onvif/restart',
        expect.objectContaining({ method: 'POST' }),
      );
    });

    it('should throw ApiError on 404 (unknown service)', async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(new Response('unknown service', {
        status: 404,
      }));

      await expect(restartService('nope')).rejects.toThrow(ApiError);
    });

    it('should throw ApiError on 503 (supervisor unreachable)', async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(new Response('unreachable', {
        status: 503,
      }));

      await expect(restartService('onvif')).rejects.toThrow(ApiError);
    });
  });
});
