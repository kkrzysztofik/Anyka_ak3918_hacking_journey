/**
 * Auth Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import { checkDeviceReachable, verifyCredentials } from '@/services/authService';
import { createMockSOAPFaultResponse, createMockSOAPResponse } from '@/test/utils';

// Mock the api module
vi.mock('@/services/api', () => ({
  apiClient: {
    post: vi.fn(),
  },
  ENDPOINTS: {
    device: '/onvif/device_service',
  },
}));

describe('authService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('verifyCredentials', () => {
    it('should return success with device info on valid response', async () => {
      const mockResponse = createMockSOAPResponse(`
        <tds:GetDeviceInformationResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
          <tds:Manufacturer>Anyka</tds:Manufacturer>
          <tds:Model>AK3918E</tds:Model>
          <tds:FirmwareVersion>1.0.0</tds:FirmwareVersion>
          <tds:SerialNumber>ABC123</tds:SerialNumber>
          <tds:HardwareId>HW001</tds:HardwareId>
        </tds:GetDeviceInformationResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await verifyCredentials('admin', 'password');

      expect(result.success).toBe(true);
      expect(result.deviceInfo).toBeDefined();
      expect(result.deviceInfo?.manufacturer).toBe('Anyka');
      expect(result.deviceInfo?.model).toBe('AK3918E');
    });

    it('should return failure on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Not Authorized');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await verifyCredentials('admin', 'wrong');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should return failure on network error', async () => {
      vi.mocked(apiClient.post).mockRejectedValueOnce(new Error('Network error'));

      const result = await verifyCredentials('admin', 'password');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should handle 401 unauthorized error', async () => {
      vi.mocked(apiClient.post).mockRejectedValueOnce(new Error('401 Unauthorized'));

      const result = await verifyCredentials('admin', 'wrongpass');

      expect(result.success).toBe(false);
      expect(result.error).toBe('Invalid username or password');
    });

    it('should handle ECONNREFUSED error', async () => {
      vi.mocked(apiClient.post).mockRejectedValueOnce(new Error('ECONNREFUSED'));

      const result = await verifyCredentials('admin', 'password');

      expect(result.success).toBe(false);
      expect(result.error).toBe('Unable to connect to camera');
    });

    it('should handle non-Error exceptions', async () => {
      vi.mocked(apiClient.post).mockRejectedValueOnce('string error');

      const result = await verifyCredentials('admin', 'password');

      expect(result.success).toBe(false);
      expect(result.error).toBe('An unexpected error occurred');
    });

    it('should call apiClient.post with device endpoint', async () => {
      const mockResponse = createMockSOAPResponse(`
        <tds:GetDeviceInformationResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
          <tds:Manufacturer>Test</tds:Manufacturer>
        </tds:GetDeviceInformationResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await verifyCredentials('admin', 'password');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('GetDeviceInformation'),
        expect.objectContaining({ headers: expect.any(Object) }),
      );
    });
  });

  describe('checkDeviceReachable', () => {
    it('should return true when device responds with 200', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce({
        status: 200,
        data: '<response/>',
      });

      const result = await checkDeviceReachable();

      expect(result).toBe(true);
    });

    it('should return false on network error', async () => {
      vi.mocked(apiClient.post).mockRejectedValueOnce(new Error('Connection refused'));

      const result = await checkDeviceReachable();

      expect(result).toBe(false);
    });
  });
});
