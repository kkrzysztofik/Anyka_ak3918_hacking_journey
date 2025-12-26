/**
 * Test utilities and mock helpers
 */
import type { AxiosResponse } from 'axios';
import { vi } from 'vitest';

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
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      headers: {} as any,
    },
  };
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
</soap:Envelope>`;
}

/**
 * Create a SOAP fault response XML
 * @param code - SOAP fault code (e.g., 'soap:Sender', 'soap:Receiver')
 * @param reason - Fault reason text
 * @returns SOAP envelope XML string with fault
 */
export function createSOAPFaultResponse(code: string, reason: string): string {
  return createTestSOAPResponse(
    `<soap:Fault>
      <soap:Code><soap:Value>${code}</soap:Value></soap:Code>
      <soap:Reason><soap:Text>${reason}</soap:Text></soap:Reason>
    </soap:Fault>`,
  );
}

/**
 * Create a SOAP success response (alias for createTestSOAPResponse)
 * @param bodyContent - XML body content
 * @returns SOAP envelope XML string
 */
export function createSOAPSuccessResponse(bodyContent: string): string {
  return createTestSOAPResponse(bodyContent);
}

/**
 * Create a full Axios response with SOAP envelope
 * @param bodyContent - XML body content
 * @param status - HTTP status code (default: 200)
 * @returns Axios response with SOAP envelope in data
 */
export function createMockSOAPResponse(bodyContent: string, status = 200): AxiosResponse<string> {
  return createMockResponse(createTestSOAPResponse(bodyContent), status);
}

/**
 * Create a full Axios response with SOAP fault
 * @param code - SOAP fault code (e.g., 'soap:Sender', 'soap:Receiver')
 * @param reason - Fault reason text
 * @param status - HTTP status code (default: 200)
 * @returns Axios response with SOAP fault in data
 */
export function createMockSOAPFaultResponse(
  code: string,
  reason: string,
  status = 200,
): AxiosResponse<string> {
  return createMockResponse(createSOAPFaultResponse(code, reason), status);
}

/**
 * Mock apiClient for service tests
 */
export function mockApiClient() {
  return {
    post: vi.fn(),
    get: vi.fn(),
  };
}
