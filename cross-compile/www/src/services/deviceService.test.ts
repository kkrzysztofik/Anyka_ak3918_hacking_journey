/**
 * Device Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import {
  getDeviceIdentification,
  getDeviceInformation,
  getScopes,
  setScopes,
} from '@/services/deviceService';
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

describe('deviceService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getDeviceInformation', () => {
    it('should parse device information correctly', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetDeviceInformationResponse>
          <Manufacturer>Anyka</Manufacturer>
          <Model>AK3918E</Model>
          <FirmwareVersion>2.0.0</FirmwareVersion>
          <SerialNumber>SN12345</SerialNumber>
          <HardwareId>HW12345</HardwareId>
        </GetDeviceInformationResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getDeviceInformation();

      expect(result.manufacturer).toBe('Anyka');
      expect(result.model).toBe('AK3918E');
      expect(result.firmwareVersion).toBe('2.0.0');
      expect(result.serialNumber).toBe('SN12345');
      expect(result.hardwareId).toBe('HW12345');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Operation failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getDeviceInformation()).rejects.toThrow();
    });

    it('should throw on missing response data', async () => {
      const mockResponse = createMockSOAPResponse('<SomeOtherResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getDeviceInformation()).rejects.toThrow('Invalid response');
    });
  });

  describe('getScopes', () => {
    it('should parse scopes and extract name/location', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetScopesResponse>
          <Scopes>
            <ScopeDef>Fixed</ScopeDef>
            <ScopeItem>onvif://www.onvif.org/name/MyCam</ScopeItem>
          </Scopes>
          <Scopes>
            <ScopeDef>Configurable</ScopeDef>
            <ScopeItem>onvif://www.onvif.org/location/LivingRoom</ScopeItem>
          </Scopes>
        </GetScopesResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getScopes();

      expect(result.name).toBe('MyCam');
      expect(result.location).toBe('LivingRoom');
    });

    it('should return empty strings when no matching scopes', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetScopesResponse>
          <Scopes>
            <ScopeItem>onvif://www.onvif.org/type/video_encoder</ScopeItem>
          </Scopes>
        </GetScopesResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getScopes();

      expect(result.name).toBe('');
      expect(result.location).toBe('');
    });
  });

  describe('setScopes', () => {
    it('should call API with correct SOAP body', async () => {
      const mockResponse = createMockSOAPResponse('<SetScopesResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setScopes('NewName', 'NewLocation');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('NewName'),
      );
      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('NewLocation'),
      );
    });

    it('should throw on failure', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Set failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(setScopes('Test', 'Test')).rejects.toThrow();
    });
  });

  describe('getDeviceIdentification', () => {
    it('should combine device info and scopes', async () => {
      const deviceInfoResponse = createMockSOAPResponse(`
        <GetDeviceInformationResponse>
          <Manufacturer>Anyka</Manufacturer>
          <Model>AK3918E</Model>
          <FirmwareVersion>1.0.0</FirmwareVersion>
          <SerialNumber>SN123</SerialNumber>
          <HardwareId>HW123</HardwareId>
        </GetDeviceInformationResponse>
      `);

      const scopesResponse = createMockSOAPResponse(`
        <GetScopesResponse>
          <Scopes><ScopeItem>onvif://www.onvif.org/name/TestCam</ScopeItem></Scopes>
          <Scopes><ScopeItem>onvif://www.onvif.org/location/Office</ScopeItem></Scopes>
        </GetScopesResponse>
      `);

      vi.mocked(apiClient.post)
        .mockResolvedValueOnce(deviceInfoResponse)
        .mockResolvedValueOnce(scopesResponse);

      const result = await getDeviceIdentification();

      expect(result.deviceInfo.manufacturer).toBe('Anyka');
      expect(result.name).toBe('TestCam');
      expect(result.location).toBe('Office');
    });
  });
});
