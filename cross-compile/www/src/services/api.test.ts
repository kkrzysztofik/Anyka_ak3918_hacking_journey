/**
 * API Client Tests
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  ApiError,
  DEFAULT_HEADERS,
  DEFAULT_TIMEOUT_MS,
  ENDPOINTS,
  apiClient,
  authorizedXhrPut,
  setAuthHeaderGetter,
} from './api';

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
      expect(DEFAULT_HEADERS['Content-Type']).toBe('application/soap+xml; charset=utf-8');
      expect(DEFAULT_HEADERS.Accept).toBe('application/soap+xml, application/xml, */*');
    });

    it('should have correct timeout', () => {
      expect(DEFAULT_TIMEOUT_MS).toBe(10000);
    });

    it('should send the default headers on a request', async () => {
      const fetchMock = vi.fn(
        async (_url: string, _init?: RequestInit) => new Response('ok', { status: 200 }),
      );
      vi.stubGlobal('fetch', fetchMock);

      await apiClient.post('/test', '<body />');

      const headers = new Headers(fetchMock.mock.calls[0][1]?.headers);
      expect(headers.get('Content-Type')).toBe('application/soap+xml; charset=utf-8');
      expect(headers.get('Accept')).toBe('application/soap+xml, application/xml, */*');
    });

    it('should let a lowercase authorization header suppress the injected one', async () => {
      const mockGetter = vi.fn().mockResolvedValue('Basic from-getter');
      setAuthHeaderGetter(mockGetter);

      const fetchMock = vi.fn(
        async (_url: string, _init?: RequestInit) => new Response('ok', { status: 200 }),
      );
      vi.stubGlobal('fetch', fetchMock);

      await apiClient.post('/test', '<body />', {
        headers: { authorization: 'Basic explicit' },
      });

      const headers = new Headers(fetchMock.mock.calls[0][1]?.headers);
      expect(headers.get('Authorization')).toBe('Basic explicit');
      expect(mockGetter).not.toHaveBeenCalled();
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

  describe('abort signal', () => {
    // AbortSignal.any() is unavailable on the declared Firefox target, so the
    // combining is hand-rolled; these cover the three ways it can resolve.
    const captureSignal = () => {
      const fetchMock = vi.fn(async (_url: string, init?: RequestInit) => {
        await new Promise((resolve) => setTimeout(resolve, 50));
        init?.signal?.throwIfAborted();
        return new Response('ok', { status: 200 });
      });
      vi.stubGlobal('fetch', fetchMock);
      return fetchMock;
    };

    it('should abort when the caller aborts', async () => {
      captureSignal();
      const controller = new AbortController();
      const pending = apiClient.post('/test', '<body />', { signal: controller.signal });
      controller.abort(new Error('caller cancelled'));

      await expect(pending).rejects.toThrow('caller cancelled');
    });

    it('should abort when the timeout fires before the caller aborts', async () => {
      captureSignal();
      const controller = new AbortController();

      await expect(
        apiClient.post('/test', '<body />', { signal: controller.signal, timeout: 1 }),
      ).rejects.toThrow(/timed out|abort/i);
    });

    it('should abort immediately when the caller signal is already aborted', async () => {
      const fetchMock = captureSignal();
      const controller = new AbortController();
      controller.abort(new Error('already gone'));

      await expect(
        apiClient.post('/test', '<body />', { signal: controller.signal }),
      ).rejects.toThrow('already gone');
      expect(fetchMock.mock.calls[0][1]?.signal?.aborted).toBe(true);
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

  describe('authorizedXhrPut', () => {
    type XhrListener = (ev?: ProgressEvent) => void;

    class MockXHR {
      static instances: MockXHR[] = [];

      readyState = 0;
      status = 0;
      responseText = '';
      upload = {
        listeners: new Map<string, XhrListener[]>(),
        addEventListener(type: string, listener: XhrListener) {
          const list = this.listeners.get(type) ?? [];
          list.push(listener);
          this.listeners.set(type, list);
        },
        dispatch(type: string, ev: ProgressEvent) {
          for (const listener of this.listeners.get(type) ?? []) {
            listener(ev);
          }
        },
      };

      private listeners = new Map<string, XhrListener[]>();
      method = '';
      url = '';
      headers: Record<string, string> = {};
      body: Blob | null = null;
      aborted = false;

      constructor() {
        MockXHR.instances.push(this);
      }

      open(method: string, url: string) {
        this.method = method;
        this.url = url;
        this.readyState = 1;
      }

      setRequestHeader(name: string, value: string) {
        this.headers[name] = value;
      }

      addEventListener(type: string, listener: XhrListener) {
        const list = this.listeners.get(type) ?? [];
        list.push(listener);
        this.listeners.set(type, list);
      }

      send(body?: Document | XMLHttpRequestBodyInit | null) {
        this.body = body instanceof Blob ? body : null;
        this.readyState = 2;
      }

      abort() {
        this.aborted = true;
        this.dispatch('abort');
      }

      dispatch(type: string, ev?: ProgressEvent) {
        for (const listener of this.listeners.get(type) ?? []) {
          listener(ev);
        }
      }

      complete(status: number, responseText: string) {
        this.status = status;
        this.responseText = responseText;
        this.readyState = 4;
        this.dispatch('load');
      }

      failNetwork() {
        this.dispatch('error');
      }
    }

    beforeEach(() => {
      MockXHR.instances = [];
      vi.stubGlobal('XMLHttpRequest', MockXHR);
    });

    it('should resolve status and bodyText on 202', async () => {
      const pending = authorizedXhrPut('/api/update', new Blob(['fw']));
      const xhr = MockXHR.instances[0];
      expect(xhr.method).toBe('PUT');
      expect(xhr.url).toBe('/api/update');

      xhr.complete(202, 'accepted');

      await expect(pending).resolves.toEqual({ status: 202, bodyText: 'accepted' });
    });

    it('should resolve non-202 status without throwing (caller decides)', async () => {
      const pending = authorizedXhrPut('/api/update', new Blob(['fw']));
      MockXHR.instances[0].complete(500, 'boom');

      await expect(pending).resolves.toEqual({ status: 500, bodyText: 'boom' });
    });

    it('should fire onProgress from upload progress events', async () => {
      const onProgress = vi.fn();
      const pending = authorizedXhrPut('/api/update', new Blob(['fw']), { onProgress });
      const xhr = MockXHR.instances[0];

      xhr.upload.dispatch('progress', {
        lengthComputable: true,
        loaded: 50,
        total: 100,
      } as ProgressEvent);
      xhr.complete(202, '');

      await pending;
      expect(onProgress).toHaveBeenCalledWith({ loaded: 50, total: 100 });
    });

    it('should reject when the abort signal fires', async () => {
      const controller = new AbortController();
      const pending = authorizedXhrPut('/api/update', new Blob(['fw']), {
        signal: controller.signal,
      });

      controller.abort();

      await expect(pending).rejects.toThrow(/abort/i);
      expect(MockXHR.instances[0].aborted).toBe(true);
    });

    it('should reject immediately when the signal is already aborted', async () => {
      const controller = new AbortController();
      controller.abort();

      await expect(
        authorizedXhrPut('/api/update', new Blob(['fw']), { signal: controller.signal }),
      ).rejects.toThrow(/abort/i);
    });

    it('should inject Authorization from the auth getter', async () => {
      const mockGetter = vi.fn().mockResolvedValue('Basic dGVzdDp0ZXN0');
      setAuthHeaderGetter(mockGetter);

      const pending = authorizedXhrPut('/api/update', new Blob(['fw']));
      // auth is awaited before open/send — flush microtasks
      await Promise.resolve();
      await Promise.resolve();

      const xhr = MockXHR.instances[0];
      expect(xhr.headers.Authorization).toBe('Basic dGVzdDp0ZXN0');
      expect(mockGetter).toHaveBeenCalled();

      xhr.complete(202, '');
      await pending;
    });

    it('should reject if aborted while awaiting auth', async () => {
      let resolveAuth!: (value: string) => void;
      setAuthHeaderGetter(
        () =>
          new Promise<string>((resolve) => {
            resolveAuth = resolve;
          }),
      );

      const controller = new AbortController();
      const pending = authorizedXhrPut('/api/update', new Blob(['fw']), {
        signal: controller.signal,
      });

      controller.abort();
      resolveAuth('Basic late');

      await expect(pending).rejects.toThrow(/abort/i);
      expect(MockXHR.instances).toHaveLength(0);
    });

    it('should clear auth and dispatch on 401', async () => {
      sessionStorage.setItem('onvif_camera_auth', 'test');
      const unauthorized = vi.fn();
      globalThis.addEventListener('auth:unauthorized', unauthorized);

      try {
        const pending = authorizedXhrPut('/api/update', new Blob(['fw']));
        MockXHR.instances[0].complete(401, 'Unauthorized');

        await expect(pending).resolves.toEqual({ status: 401, bodyText: 'Unauthorized' });
        expect(sessionStorage.getItem('onvif_camera_auth')).toBeNull();
        expect(unauthorized).toHaveBeenCalled();
      } finally {
        globalThis.removeEventListener('auth:unauthorized', unauthorized);
      }
    });
  });
});
