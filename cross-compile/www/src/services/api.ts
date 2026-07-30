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
  const signal = config.signal ? AbortSignal.any([config.signal, timeout]) : timeout;

  const response = await fetch(url, {
    method: 'POST',
    headers,
    body,
    signal,
  });

  const text = await response.text();

  if (response.status === 401) {
    sessionStorage.removeItem('onvif_camera_auth');
    globalThis.dispatchEvent(new CustomEvent('auth:unauthorized'));
  }
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
