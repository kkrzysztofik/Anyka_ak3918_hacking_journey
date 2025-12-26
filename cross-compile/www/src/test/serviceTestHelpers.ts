/**
 * Service test helpers for standardized test setup and patterns
 */
import { beforeEach } from 'vitest';

import { createMockSOAPFaultResponse, createMockSOAPResponse } from './utils';

/**
 * Setup standard test suite for service tests
 * This provides beforeEach hook that clears all mocks
 * @param setupFn - Optional setup function to run before each test
 */
export function setupServiceTestSuite(setupFn?: () => void): void {
  beforeEach(() => {
    // Note: vi.clearAllMocks() is called in individual test files
    // This function can be extended for additional setup if needed
    if (setupFn) {
      setupFn();
    }
  });
}

/**
 * Create a mock SOAP success response for service tests
 * @param bodyContent - XML body content
 * @returns Axios response with SOAP envelope
 */
export function createServiceMockResponse(bodyContent: string) {
  return createMockSOAPResponse(bodyContent);
}

/**
 * Create a mock SOAP fault response for service tests
 * @param code - SOAP fault code (default: 'soap:Sender')
 * @param reason - Fault reason text (default: 'Operation failed')
 * @returns Axios response with SOAP fault
 */
export function createServiceFaultResponse(code = 'soap:Sender', reason = 'Operation failed') {
  return createMockSOAPFaultResponse(code, reason);
}

/**
 * Common test patterns for service tests
 */
export const serviceTestPatterns = {
  /**
   * Test pattern for successful SOAP response
   */
  successResponse: (bodyContent: string) => createMockSOAPResponse(bodyContent),

  /**
   * Test pattern for SOAP fault response
   */
  faultResponse: (code = 'soap:Sender', reason = 'Operation failed') =>
    createMockSOAPFaultResponse(code, reason),

  /**
   * Test pattern for network error
   */
  networkError: (message = 'Network error') => {
    const error = new Error(message);
    return Promise.reject(error);
  },
};
