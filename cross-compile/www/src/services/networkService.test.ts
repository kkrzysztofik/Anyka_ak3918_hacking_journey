/**
 * Network Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient, authorizedFetch } from '@/services/api';
import {
  getDNS,
  getNetworkConfig,
  getNetworkDefaultGateway,
  getNetworkInterfaces,
  getNetworkOverlay,
  getNetworkProtocols,
  putNetworkOverlay,
  setDNS,
  setNetworkInterface,
} from '@/services/networkService';
import { createMockSOAPFaultResponse, createMockSOAPResponse } from '@/test/utils';

vi.mock('@/services/api', () => ({
  apiClient: {
    post: vi.fn(),
  },
  authorizedFetch: vi.fn(),
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
      const mockResponse = createMockSOAPResponse(`
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
            <Link>
              <OperSettings>
                <Speed>100</Speed>
              </OperSettings>
            </Link>
          </NetworkInterfaces>
        </GetNetworkInterfacesResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getNetworkInterfaces();

      expect(result).toHaveLength(1);
      expect(result[0].token).toBe('eth0');
      expect(result[0].dhcp).toBe(false);
      expect(result[0].address).toBe('192.168.1.100');
    });
  });

  describe('getNetworkDefaultGateway', () => {
    it('returns the first IPv4 gateway', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce(
        createMockSOAPResponse(`
        <GetNetworkDefaultGatewayResponse>
          <NetworkGateway>
            <IPv4Address>192.168.2.1</IPv4Address>
          </NetworkGateway>
        </GetNetworkDefaultGatewayResponse>
      `),
      );

      await expect(getNetworkDefaultGateway()).resolves.toBe('192.168.2.1');
    });
  });

  describe('getNetworkProtocols', () => {
    it('reads HTTP and RTSP ports', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce(
        createMockSOAPResponse(`
        <GetNetworkProtocolsResponse>
          <NetworkProtocols>
            <Name>HTTP</Name>
            <Port>8080</Port>
          </NetworkProtocols>
          <NetworkProtocols>
            <Name>RTSP</Name>
            <Port>8554</Port>
          </NetworkProtocols>
        </GetNetworkProtocolsResponse>
      `),
      );

      await expect(getNetworkProtocols()).resolves.toEqual({ http: 8080, rtsp: 8554 });
    });
  });

  describe('getNetworkConfig', () => {
    it('populates the gateway from GetNetworkDefaultGateway', async () => {
      vi.mocked(apiClient.post)
        .mockResolvedValueOnce(
          createMockSOAPResponse(`
          <GetNetworkInterfacesResponse>
            <NetworkInterfaces token="eth0">
              <Enabled>true</Enabled>
              <Info><Name>eth0</Name><HwAddress>00:11:22:33:44:55</HwAddress></Info>
              <IPv4>
                <Enabled>true</Enabled>
                <Config><DHCP>true</DHCP></Config>
              </IPv4>
            </NetworkInterfaces>
          </GetNetworkInterfacesResponse>
        `),
        )
        .mockResolvedValueOnce(
          createMockSOAPResponse(`
          <GetDNSResponse>
            <DNSInformation><FromDHCP>true</FromDHCP></DNSInformation>
          </GetDNSResponse>
        `),
        )
        .mockResolvedValueOnce(
          createMockSOAPResponse(`
          <GetNetworkDefaultGatewayResponse>
            <NetworkGateway><IPv4Address>192.168.2.1</IPv4Address></NetworkGateway>
          </GetNetworkDefaultGatewayResponse>
        `),
        )
        .mockResolvedValueOnce(
          createMockSOAPResponse(`
          <GetNetworkProtocolsResponse>
            <NetworkProtocols><Name>HTTP</Name><Port>80</Port></NetworkProtocols>
            <NetworkProtocols><Name>RTSP</Name><Port>554</Port></NetworkProtocols>
          </GetNetworkProtocolsResponse>
        `),
        );

      const config = await getNetworkConfig();
      expect(config.interfaces[0].gateway).toBe('192.168.2.1');
      expect(config.protocols).toEqual({ http: 80, rtsp: 554 });
    });

    it('leaves the gateway empty when the device reports none', async () => {
      vi.mocked(apiClient.post)
        .mockResolvedValueOnce(createMockSOAPResponse('<GetNetworkInterfacesResponse />'))
        .mockResolvedValueOnce(
          createMockSOAPResponse('<GetDNSResponse><DNSInformation/></GetDNSResponse>'),
        )
        .mockResolvedValueOnce(createMockSOAPResponse('<GetNetworkDefaultGatewayResponse />'))
        .mockResolvedValueOnce(createMockSOAPResponse('<GetNetworkProtocolsResponse />'));

      const config = await getNetworkConfig();
      expect(config.interfaces[0]).toBeUndefined();
    });
  });

  describe('overlay client', () => {
    it('returns overlay state from GET /api/network', async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            pending: { has_password: false, ssid: 'Net' },
            has_pending: true,
            last_failure: null,
          }),
          { status: 200 },
        ),
      );

      const state = await getNetworkOverlay();
      expect(state.pending.ssid).toBe('Net');
      expect(state.has_pending).toBe(true);
    });

    it('rejects when PUT /api/network fails', async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(
        new Response('bad request', { status: 400 }),
      );

      await expect(putNetworkOverlay({ ssid: '' })).rejects.toThrow(/bad request|400/i);
    });
  });

  describe('getDNS', () => {
    it('should parse DNS configuration', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetDNSResponse>
          <DNSInformation>
            <FromDHCP>false</FromDHCP>
            <DNSManual>
              <IPv4Address>8.8.8.8</IPv4Address>
            </DNSManual>
          </DNSInformation>
        </GetDNSResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getDNS();
      expect(result.fromDHCP).toBe(false);
      expect(result.dnsServers).toContain('8.8.8.8');
    });
  });

  describe('setNetworkInterface', () => {
    it('should send DHCP configuration', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce(
        createMockSOAPResponse('<SetNetworkInterfacesResponse />'),
      );

      await setNetworkInterface('eth0', true);

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tt:DHCP>true</tt:DHCP>'),
      );
    });
  });

  describe('setDNS', () => {
    it('should send DNS from DHCP configuration', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce(createMockSOAPResponse('<SetDNSResponse />'));

      await setDNS(true);

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:FromDHCP>true</tds:FromDHCP>'),
      );
    });
  });
});
