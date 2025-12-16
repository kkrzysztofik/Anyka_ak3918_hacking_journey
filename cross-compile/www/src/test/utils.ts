/**
 * Test utilities and mock helpers
 */

import { vi } from 'vitest'
import type { AxiosResponse } from 'axios'

/**
 * Create a mock Axios response
 */
export function createMockResponse<T>(data: T, status = 200): AxiosResponse<T> {
  return {
    data,
    status,
    statusText: 'OK',
    headers: {},
    config: {
      headers: {} as any,
    },
  }
}

/**
 * Create a SOAP envelope for testing
 */
export function createTestSOAPResponse(bodyContent: string): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
  <soap:Body>
    ${bodyContent}
  </soap:Body>
</soap:Envelope>`
}

/**
 * Mock apiClient for service tests
 */
export function mockApiClient() {
  return {
    post: vi.fn(),
    get: vi.fn(),
  }
}
