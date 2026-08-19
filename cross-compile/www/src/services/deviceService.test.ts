/**
 * Device Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import {
  type Scope,
  getDeviceIdentification,
  getDeviceInformation,
  getDiscoveryMode,
  getHostname,
  getScopes,
  nameFromScopes,
  scopesForSave,
  setDiscoveryMode,
  setHostname,
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

      await expect(getDeviceInformation()).rejects.toThrow(
        'SOAP response target "GetDeviceInformationResponse" not found',
      );
    });
  });

  describe('getScopes', () => {
    it('should parse scope definitions from GetScopes', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetScopesResponse>
          <Scopes>
            <ScopeDef>Fixed</ScopeDef>
            <ScopeItem>onvif://www.onvif.org/type/ptz</ScopeItem>
          </Scopes>
          <Scopes>
            <ScopeDef>Configurable</ScopeDef>
            <ScopeItem>onvif://www.onvif.org/name/Front%20Door</ScopeItem>
          </Scopes>
        </GetScopesResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const scopes = await getScopes();

      expect(scopes).toHaveLength(2);
      expect(scopes[0].scopeDef).toBe('Fixed');
      expect(scopes[0].scopeItem).toBe('onvif://www.onvif.org/type/ptz');
    });
  });

  describe('nameFromScopes', () => {
    it.each([
      ['onvif://www.onvif.org/name/Front%20Door', 'Front Door'],
      ['onvif://www.onvif.org/name/Cam', 'Cam'],
    ])('should decode %s to %s', (scopeItem, expected) => {
      expect(nameFromScopes([{ scopeDef: 'Configurable', scopeItem }])).toBe(expected);
    });

    it('should return empty string when no name scope is present', () => {
      expect(nameFromScopes([])).toBe('');
    });
  });

  describe('setScopes', () => {
    it('should preserve scopes it does not manage when saving', async () => {
      // A scope no form field represents — added by an ONVIF client, or the default
      // location/country scope. The old two-argument setScopes() destroyed these.
      const existing: Scope[] = [
        { scopeDef: 'Configurable', scopeItem: 'onvif://www.onvif.org/name/Old' },
        { scopeDef: 'Configurable', scopeItem: 'onvif://www.onvif.org/location/country/unknown' },
        { scopeDef: 'Fixed', scopeItem: 'onvif://www.onvif.org/type/video_encoder' },
      ];

      vi.mocked(apiClient.post).mockResolvedValueOnce(
        createMockSOAPResponse('<SetScopesResponse />'),
      );

      await setScopes(scopesForSave(existing, { name: 'New', location: 'Hall' }));

      const sentBody = String(vi.mocked(apiClient.post).mock.calls[0]?.[1]);
      expect(sentBody).toContain('location/country/unknown');
      expect(sentBody).toContain('name/New');
      expect(sentBody).not.toContain('type/video_encoder');
    });

    it('should call API with correct SOAP body', async () => {
      const mockResponse = createMockSOAPResponse('<SetScopesResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setScopes(scopesForSave([], { name: 'NewName', location: 'NewLocation' }));

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('NewName'),
      );
      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('NewLocation'),
      );
    });

    it('should XML-escape each scope item before interpolating', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce(
        createMockSOAPResponse('<SetScopesResponse />'),
      );

      await setScopes(['onvif://www.onvif.org/name/A&B']);

      const sentBody = String(vi.mocked(apiClient.post).mock.calls[0]?.[1]);
      expect(sentBody).toContain('<tds:Scopes>onvif://www.onvif.org/name/A&amp;B</tds:Scopes>');
      expect(sentBody).not.toContain('<tds:Scopes>onvif://www.onvif.org/name/A&B</tds:Scopes>');
    });

    it('should throw on failure', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Set failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(
        setScopes(scopesForSave([], { name: 'Test', location: 'Test' })),
      ).rejects.toThrow();
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

  describe('getDiscoveryMode', () => {
    it('should parse the discovery mode', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce(
        createMockSOAPResponse(`
          <GetDiscoveryModeResponse>
            <DiscoveryMode>NonDiscoverable</DiscoveryMode>
          </GetDiscoveryModeResponse>
        `),
      );

      await expect(getDiscoveryMode()).resolves.toBe('NonDiscoverable');
    });
  });

  describe('setDiscoveryMode', () => {
    it('should send the discovery mode', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce(
        createMockSOAPResponse('<SetDiscoveryModeResponse />'),
      );

      await setDiscoveryMode('NonDiscoverable');

      const sentBody = String(vi.mocked(apiClient.post).mock.calls[0]?.[1]);
      expect(sentBody).toContain('NonDiscoverable');
    });
  });

  describe('getHostname', () => {
    it('should parse the hostname', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce(
        createMockSOAPResponse(`
          <GetHostnameResponse>
            <HostnameInformation>
              <Name>ipcam</Name>
            </HostnameInformation>
          </GetHostnameResponse>
        `),
      );

      await expect(getHostname()).resolves.toBe('ipcam');
    });
  });

  describe('setHostname', () => {
    it('should send the hostname', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce(
        createMockSOAPResponse('<SetHostnameResponse />'),
      );

      await setHostname('ipcam');

      const sentBody = String(vi.mocked(apiClient.post).mock.calls[0]?.[1]);
      expect(sentBody).toContain('ipcam');
    });
  });
});
