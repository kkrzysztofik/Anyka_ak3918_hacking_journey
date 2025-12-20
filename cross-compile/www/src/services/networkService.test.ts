/**
 * Network Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import {
  getDNS,
  getNetworkInterfaces,
  setDNS,
  setNetworkInterface,
} from '@/services/networkService';

// Mock the api module
vi.mock('@/services/api', () => ({
  apiClient: {
    post: vi.fn(),
  },
  ENDPOINTS: {
    device: '/onvif/device_service',
  },
}));

describe('networkService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getNetworkInterfaces', () => {
    it('should parse network interfaces correctly', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <GetNetworkInterfacesResponse>
                <NetworkInterfaces token="eth0">
                  <Enabled>true</Enabled>
                  <Info>
                    <Name>eth0</Name>
                    <HwAddress>00:11:22:33:44:55</HwAddress>
                  </Info>
                  <IPv4>
                    <Enabled>true</Enabled>
                    <Config>
                      <DHCP>false</DHCP>
                      <Manual>
                        <Address>192.168.1.100</Address>
                        <PrefixLength>24</PrefixLength>
                      </Manual>
                    </Config>
                  </IPv4>
                </NetworkInterfaces>
              </GetNetworkInterfacesResponse>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getNetworkInterfaces();

      expect(result).toHaveLength(1);
      expect(result[0].token).toBe('eth0');
      expect(result[0].name).toBe('eth0');
      expect(result[0].hwAddress).toBe('00:11:22:33:44:55');
      expect(result[0].dhcp).toBe(false);
      expect(result[0].address).toBe('192.168.1.100');
      expect(result[0].prefixLength).toBe(24);
    });

    it('should return empty array when no interfaces', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <GetNetworkInterfacesResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getNetworkInterfaces();

      expect(result).toEqual([]);
    });
  });

  describe('getDNS', () => {
    it('should parse DNS configuration', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <GetDNSResponse>
                <DNSInformation>
                  <FromDHCP>false</FromDHCP>
                  <DNSManual>
                    <IPv4Address>8.8.8.8</IPv4Address>
                  </DNSManual>
                </DNSInformation>
              </GetDNSResponse>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getDNS();

      expect(result.fromDHCP).toBe(false);
      expect(result.dnsServers).toContain('8.8.8.8');
    });
  });

  describe('setNetworkInterface', () => {
    it('should send DHCP configuration', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <SetNetworkInterfacesResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setNetworkInterface('eth0', true);

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tt:DHCP>true</tt:DHCP>'),
      );
    });

    it('should send static IP configuration', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <SetNetworkInterfacesResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setNetworkInterface('eth0', false, '192.168.1.50', 24);

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('192.168.1.50'),
      );
    });

    it('should throw on failure', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <soap:Fault>
                <soap:Code><soap:Value>soap:Sender</soap:Value></soap:Code>
                <soap:Reason><soap:Text>Failed</soap:Text></soap:Reason>
              </soap:Fault>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(setNetworkInterface('eth0', true)).rejects.toThrow();
    });
  });

  describe('setDNS', () => {
    it('should send DNS from DHCP configuration', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <SetDNSResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setDNS(true);

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:FromDHCP>true</tds:FromDHCP>'),
      );
    });

    it('should send manual DNS servers', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <SetDNSResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setDNS(false, ['8.8.8.8', '8.8.4.4']);

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('8.8.8.8'),
      );
      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('8.8.4.4'),
      );
    });

    it('should throw on failure', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <soap:Fault>
                <soap:Code><soap:Value>soap:Sender</soap:Value></soap:Code>
                <soap:Reason><soap:Text>DNS Failed</soap:Text></soap:Reason>
              </soap:Fault>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);
    });
  });
});
