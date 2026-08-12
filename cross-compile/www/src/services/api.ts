/**
 * API Client with Basic Auth
 *
 * Fetch-based client for ONVIF SOAP requests with automatic Basic Auth
 * header injection when a getter is registered.
 */

// Store reference to auth getter (set by AuthProvider integration)
let getAuthHeader: (() => Promise<string | null>) | null = null;

/**
 * Set the auth header getter function
 * Called by App.tsx after AuthProvider is mounted
 */
export function setAuthHeaderGetter(getter: (() => Promise<string | null>) | null) {
  getAuthHeader = getter;
}

export const DEFAULT_HEADERS: Readonly<Record<string, string>> = {
  'Content-Type': 'application/soap+xml; charset=utf-8',
  Accept: 'application/soap+xml, application/xml, */*',
};

export const DEFAULT_TIMEOUT_MS = 10_000;

export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly data: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export interface ApiResponse<T = string> {
  data: T;
  status: number;
}

export interface ApiRequestConfig {
  headers?: Record<string, string>;
  timeout?: number;
  signal?: AbortSignal;
}

/**
 * Abort as soon as either input signal aborts, preserving the reason.
 *
 * `AbortSignal.any()` would do this in one line but needs Firefox 124, and the
 * build targets Firefox 119 (see the browser list in vite.config.ts). Listeners
 * are registered against the output signal, so aborting removes them.
 */
function raceSignals(a: AbortSignal, b: AbortSignal): AbortSignal {
  const controller = new AbortController();
  for (const source of [a, b]) {
    if (source.aborted) {
      controller.abort(source.reason);
      return controller.signal;
    }
    source.addEventListener('abort', () => controller.abort(source.reason), {
      signal: controller.signal,
    });
  }
  return controller.signal;
}

/**
 * Fetch with the same auth injection and 401 handling as SOAP posts.
 *
 * The diagnostics endpoints are plain JSON GETs, but they must share the
 * session-expiry path: a poll that quietly 401s every 5 s would leave a dead
 * page with no sign-in prompt.
 */
export async function authorizedFetch(
  url: string,
  init: RequestInit = {},
  config: ApiRequestConfig = {},
): Promise<Response> {
  const headers = new Headers(init.headers);
  for (const [name, value] of Object.entries(config.headers ?? {})) {
    headers.set(name, value);
  }

  if (!headers.has('Authorization') && getAuthHeader) {
    const authHeader = await getAuthHeader();
    if (authHeader) {
      headers.set('Authorization', authHeader);
    }
  }

  const timeoutMs = config.timeout ?? DEFAULT_TIMEOUT_MS;
  const timeout = AbortSignal.timeout(timeoutMs);
  const signal = init.signal ? raceSignals(init.signal as AbortSignal, timeout) : timeout;

  const response = await fetch(url, { ...init, headers, signal });

  if (response.status === 401) {
    sessionStorage.removeItem('onvif_camera_auth');
    globalThis.dispatchEvent(new CustomEvent('auth:unauthorized'));
  }

  return response;
}

export type UploadProgress = { loaded: number; total: number };

/**
 * PUT via XHR for upload progress. Same auth injection and 401 session-expiry
 * path as {@link authorizedFetch}; no default timeout (firmware bundles are large).
 *
 * Always resolves with status + body — callers (e.g. uploadFirmware) decide
 * which statuses are success vs ApiError.
 */
export function authorizedXhrPut(
  url: string,
  body: Blob,
  options: {
    onProgress?: (p: UploadProgress) => void;
    signal?: AbortSignal;
  } = {},
): Promise<{ status: number; bodyText: string }> {
  const { onProgress, signal } = options;

  return (async () => {
    const abortError = () =>
      signal?.reason instanceof Error
        ? signal.reason
        : new DOMException('The operation was aborted.', 'AbortError');

    if (signal?.aborted) {
      throw abortError();
    }

    let authHeader: string | null = null;
    if (getAuthHeader) {
      authHeader = await getAuthHeader();
    }

    // Auth await can race with abort; re-check before open/send.
    if (signal?.aborted) {
      throw abortError();
    }

    const xhr = new XMLHttpRequest();

    return await new Promise<{ status: number; bodyText: string }>((resolve, reject) => {
      const onAbort = () => {
        xhr.abort();
        reject(abortError());
      };

      if (signal) {
        signal.addEventListener('abort', onAbort, { once: true });
      }

      xhr.upload.addEventListener('progress', (ev) => {
        if (onProgress && ev.lengthComputable) {
          onProgress({ loaded: ev.loaded, total: ev.total });
        }
      });

      xhr.addEventListener('load', () => {
        signal?.removeEventListener('abort', onAbort);
        if (xhr.status === 401) {
          sessionStorage.removeItem('onvif_camera_auth');
          globalThis.dispatchEvent(new CustomEvent('auth:unauthorized'));
        }
        resolve({ status: xhr.status, bodyText: xhr.responseText });
      });

      xhr.addEventListener('error', () => {
        signal?.removeEventListener('abort', onAbort);
        reject(new TypeError('Network request failed'));
      });

      xhr.addEventListener('abort', () => {
        signal?.removeEventListener('abort', onAbort);
        reject(abortError());
      });

      xhr.open('PUT', url);
      if (authHeader) {
        xhr.setRequestHeader('Authorization', authHeader);
      }
      xhr.send(body);
    });
  })();
}

/**
 * Configured fetch client for ONVIF SOAP requests
 *
 * Headers:
 * - Content-Type: SOAP 1.2 XML format with UTF-8 encoding
 * - Accept: Indicates server can respond with SOAP/XML formats
 *
 * Note on Brotli: build output is pre-compressed to .br and .gz by
 * scripts/precompress.mjs, and onvif-rust serves them via ServeDir's
 * precompressed_br()/precompressed_gzip(). Browsers negotiate via
 * Accept-Encoding; no client-side handling is needed here.
 */
async function request(
  url: string,
  body: string,
  config: ApiRequestConfig = {},
): Promise<ApiResponse> {
  // Headers, not a plain object: header names are case-insensitive, so a caller
  // passing `authorization` must suppress the injected `Authorization` rather
  // than end up with both merged into one comma-joined value.
  const headers = new Headers(DEFAULT_HEADERS);
  if (config.headers) {
    for (const [name, value] of Object.entries(config.headers)) {
      headers.set(name, value);
    }
  }
  const response = await authorizedFetch(
    url,
    {
      method: 'POST',
      headers,
      body,
      signal: config.signal,
    },
    { timeout: config.timeout },
  );

  const text = await response.text();

  if (!response.ok) {
    throw new ApiError(`Request failed with status ${response.status}`, response.status, text);
  }
  return { data: text, status: response.status };
}

export const apiClient: {
  post: (url: string, body: string, config?: ApiRequestConfig) => Promise<ApiResponse>;
} = {
  post: request,
};

/**
 * ONVIF service endpoints
 */
export const ENDPOINTS = {
  device: '/onvif/device_service',
  media: '/onvif/media_service',
  imaging: '/onvif/imaging_service',
  ptz: '/onvif/ptz_service',
} as const;

export type ServiceEndpoint = keyof typeof ENDPOINTS;
