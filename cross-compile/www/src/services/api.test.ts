/**
 * API Client Tests
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError, ENDPOINTS, apiClient, setAuthHeaderGetter } from './api';

describe('api', () => {
  beforeEach(() => {
    setAuthHeaderGetter(null);
    vi.unstubAllGlobals();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    setAuthHeaderGetter(null);
  });

  describe('ENDPOINTS', () => {
    it('should have all required endpoints', () => {
      expect(ENDPOINTS.device).toBe('/onvif/device_service');
      expect(ENDPOINTS.media).toBe('/onvif/media_service');
      expect(ENDPOINTS.imaging).toBe('/onvif/imaging_service');
      expect(ENDPOINTS.ptz).toBe('/onvif/ptz_service');
    });
  });

  describe('setAuthHeaderGetter', () => {
    it('should set auth header getter function', async () => {
      // NOSONAR: S8136 - Test credential for unit testing only
      const mockGetter = vi.fn().mockResolvedValue('Basic dGVzdDp0ZXN0'); // NOSONAR
      setAuthHeaderGetter(mockGetter);

      const authHeader = await mockGetter();
      expect(authHeader).toBe('Basic dGVzdDp0ZXN0');
      expect(mockGetter).toHaveBeenCalled();
    });

    it('should allow setting null getter', () => {
      expect(() => setAuthHeaderGetter(null)).not.toThrow();
    });
  });

  describe('apiClient configuration', () => {
    it('should have correct default headers', () => {
      expect(apiClient.defaults.headers['Content-Type']).toBe(
        'application/soap+xml; charset=utf-8',
      );
      expect(apiClient.defaults.headers.Accept).toBe('application/soap+xml, application/xml, */*');
    });

    it('should have correct timeout', () => {
      expect(apiClient.defaults.timeout).toBe(10000);
    });
  });

  describe('request auth injection', () => {
    it('should inject auth header when getter is set', async () => {
      const mockGetter = vi.fn().mockResolvedValue('Basic YWRtaW46cGFzc3dvcmQ=');
      setAuthHeaderGetter(mockGetter);

      let capturedAuth: string | null = null;
      vi.stubGlobal(
        'fetch',
        vi.fn(async (_url: string, init?: RequestInit) => {
          const headers = new Headers(init?.headers);
          capturedAuth = headers.get('Authorization');
          return new Response('ok', { status: 200 });
        }),
      );

      await apiClient.post('/test', '<body />');

      // NOSONAR: S8136 - Test credential for unit testing only
      expect(capturedAuth).toBe('Basic YWRtaW46cGFzc3dvcmQ='); // NOSONAR
      expect(mockGetter).toHaveBeenCalled();
    });

    it('should not inject auth header when getter returns null', async () => {
      const mockGetter = vi.fn().mockResolvedValue(null);
      setAuthHeaderGetter(mockGetter);

      let capturedAuth: string | null = null;
      vi.stubGlobal(
        'fetch',
        vi.fn(async (_url: string, init?: RequestInit) => {
          const headers = new Headers(init?.headers);
          capturedAuth = headers.get('Authorization');
          return new Response('ok', { status: 200 });
        }),
      );

      await apiClient.post('/test', '<body />');

      expect(capturedAuth).toBeNull();
      expect(mockGetter).toHaveBeenCalled();
    });

    it('should not inject auth header when getter is not set', async () => {
      let capturedAuth: string | null = null;
      vi.stubGlobal(
        'fetch',
        vi.fn(async (_url: string, init?: RequestInit) => {
          const headers = new Headers(init?.headers);
          capturedAuth = headers.get('Authorization');
          return new Response('ok', { status: 200 });
        }),
      );

      await apiClient.post('/test', '<body />');

      expect(capturedAuth).toBeNull();
    });

    it('should prefer explicit Authorization over getter', async () => {
      const mockGetter = vi.fn().mockResolvedValue('Basic from-getter');
      setAuthHeaderGetter(mockGetter);

      let capturedAuth: string | null = null;
      vi.stubGlobal(
        'fetch',
        vi.fn(async (_url: string, init?: RequestInit) => {
          const headers = new Headers(init?.headers);
          capturedAuth = headers.get('Authorization');
          return new Response('ok', { status: 200 });
        }),
      );

      await apiClient.post('/test', '<body />', {
        headers: { Authorization: 'Basic explicit' },
      });

      expect(capturedAuth).toBe('Basic explicit');
      expect(mockGetter).not.toHaveBeenCalled();
    });

    it('should reject when auth getter throws', async () => {
      const mockGetter = vi.fn().mockRejectedValue(new Error('Auth error'));
      setAuthHeaderGetter(mockGetter);

      vi.stubGlobal(
        'fetch',
        vi.fn(async () => new Response('ok', { status: 200 })),
      );

      await expect(apiClient.post('/test', '<body />')).rejects.toThrow('Auth error');
      expect(mockGetter).toHaveBeenCalled();
    });
  });

  describe('response handling', () => {
    it('should return data and status on success', async () => {
      vi.stubGlobal(
        'fetch',
        vi.fn(async () => new Response('<soap />', { status: 200 })),
      );

      const result = await apiClient.post('/test', '<body />');
      expect(result.status).toBe(200);
      expect(result.data).toBe('<soap />');
    });

    it('should clear auth and dispatch on 401', async () => {
      sessionStorage.setItem('onvif_camera_auth', 'test');
      const unauthorized = vi.fn();
      globalThis.addEventListener('auth:unauthorized', unauthorized);

      vi.stubGlobal(
        'fetch',
        vi.fn(async () => new Response('Unauthorized', { status: 401 })),
      );

      try {
        await expect(apiClient.post('/test', '<body />')).rejects.toBeInstanceOf(ApiError);
        expect(sessionStorage.getItem('onvif_camera_auth')).toBeNull();
        expect(unauthorized).toHaveBeenCalled();
      } finally {
        globalThis.removeEventListener('auth:unauthorized', unauthorized);
      }
    });

    it('should throw ApiError on non-401 HTTP errors', async () => {
      vi.stubGlobal(
        'fetch',
        vi.fn(async () => new Response('Server Error', { status: 500 })),
      );

      await expect(apiClient.post('/test', '<body />')).rejects.toEqual(
        expect.objectContaining({
          name: 'ApiError',
          status: 500,
          data: 'Server Error',
        }),
      );
    });

    it('should propagate network errors', async () => {
      vi.stubGlobal(
        'fetch',
        vi.fn(async () => {
          throw new Error('Network Error');
        }),
      );

      await expect(apiClient.post('/test', '<body />')).rejects.toThrow('Network Error');
    });
  });
});
