/**
 * Maintenance Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import { setSystemFactoryDefault, systemReboot } from '@/services/maintenanceService';

// Mock the api module
vi.mock('@/services/api', () => ({
  apiClient: {
    post: vi.fn(),
  },
  ENDPOINTS: {
    device: '/onvif/device_service',
  },
}));

describe('maintenanceService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('systemReboot', () => {
    it('should send reboot request', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <SystemRebootResponse>
                <Message>Rebooting...</Message>
              </SystemRebootResponse>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await systemReboot();

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('tds:SystemReboot'),
      );
    });

    it('should throw on failure', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <soap:Fault>
                <soap:Code><soap:Value>soap:Sender</soap:Value></soap:Code>
                <soap:Reason><soap:Text>Reboot failed</soap:Text></soap:Reason>
              </soap:Fault>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(systemReboot()).rejects.toThrow();
    });
  });

  describe('setSystemFactoryDefault', () => {
    it('should send soft reset request', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <SetSystemFactoryDefaultResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setSystemFactoryDefault('Soft');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:FactoryDefault>Soft</tds:FactoryDefault>'),
      );
    });

    it('should send hard reset request', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <SetSystemFactoryDefaultResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setSystemFactoryDefault('Hard');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:FactoryDefault>Hard</tds:FactoryDefault>'),
      );
    });
  });
});
