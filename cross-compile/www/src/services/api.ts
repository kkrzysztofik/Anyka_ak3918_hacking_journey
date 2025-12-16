/**
 * API Client with Basic Auth Interceptor
 *
 * Axios instance configured with automatic Basic Auth header injection.
 */

import axios, { type AxiosInstance, type AxiosError, type InternalAxiosRequestConfig } from 'axios'

// Store reference to auth getter (set by AuthProvider integration)
let getAuthHeader: (() => string | null) | null = null

/**
 * Set the auth header getter function
 * Called by App.tsx after AuthProvider is mounted
 */
export function setAuthHeaderGetter(getter: () => string | null) {
  getAuthHeader = getter
}

/**
 * Configured Axios instance for ONVIF SOAP requests
 */
export const apiClient: AxiosInstance = axios.create({
  timeout: 10000,
  headers: {
    'Content-Type': 'application/soap+xml; charset=utf-8',
  },
})

// Request interceptor - inject Basic Auth header
apiClient.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    if (getAuthHeader) {
      const authHeader = getAuthHeader()
      if (authHeader) {
        config.headers.Authorization = authHeader
      }
    }
    return config
  },
  (error: AxiosError) => Promise.reject(error)
)

// Response interceptor - handle common errors
apiClient.interceptors.response.use(
  (response) => response,
  (error: AxiosError) => {
    if (error.response?.status === 401) {
      // Clear credentials on 401 Unauthorized
      console.warn('API returned 401 Unauthorized')
    }
    return Promise.reject(error)
  }
)

/**
 * ONVIF service endpoints
 */
export const ENDPOINTS = {
  device: '/onvif/device_service',
  media: '/onvif/media_service',
  imaging: '/onvif/imaging_service',
  ptz: '/onvif/ptz_service',
} as const

export type ServiceEndpoint = keyof typeof ENDPOINTS
