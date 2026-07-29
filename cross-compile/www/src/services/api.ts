/**
 * API Client with Basic Auth Interceptor
 *
 * Axios instance configured with automatic Basic Auth header injection.
 */
import axios, { type AxiosError, type AxiosInstance, type InternalAxiosRequestConfig } from 'axios';

// Store reference to auth getter (set by AuthProvider integration)
let getAuthHeader: (() => Promise<string | null>) | null = null;

/**
 * Set the auth header getter function
 * Called by App.tsx after AuthProvider is mounted
 */
export function setAuthHeaderGetter(getter: () => Promise<string | null>) {
  getAuthHeader = getter;
}

/**
 * Configured Axios instance for ONVIF SOAP requests
 *
 * Headers:
 * - Content-Type: SOAP 1.2 XML format with UTF-8 encoding
 * - Accept: Indicates server can respond with SOAP/XML formats
 * - Accept-Encoding: Allows server to compress response with gzip or deflate
 *
 * Note on Brotli: build output is pre-compressed to .br and .gz by
 * scripts/precompress.mjs, and onvif-rust serves them via ServeDir's
 * precompressed_br()/precompressed_gzip(). Browsers negotiate via
 * Accept-Encoding; no client-side handling is needed here.
 */
export const apiClient: AxiosInstance = axios.create({
  timeout: 10000,
  headers: {
    'Content-Type': 'application/soap+xml; charset=utf-8',
    Accept: 'application/soap+xml, application/xml, */*',
  },
});

// Request interceptor - inject Basic Auth header
apiClient.interceptors.request.use(
  async (config: InternalAxiosRequestConfig) => {
    if (getAuthHeader) {
      const authHeader = await getAuthHeader();
      if (authHeader) {
        config.headers.Authorization = authHeader;
      }
    }
    return config;
  },
  (error: AxiosError) => Promise.reject(error),
);

// Response interceptor - handle common errors
apiClient.interceptors.response.use(
  (response) => response,
  (error: AxiosError) => {
    if (error.response?.status === 401) {
      sessionStorage.removeItem('onvif_camera_auth');
      globalThis.dispatchEvent(new CustomEvent('auth:unauthorized'));
    }
    return Promise.reject(error);
  },
);

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
