/**
 * Network Service
 *
 * SOAP operations for network configuration.
 */
import { ENDPOINTS, apiClient } from '@/services/api';
import { createSOAPEnvelope, parseSOAPResponse } from '@/services/soap/client';

export interface NetworkInterface {
  token: string;
  enabled: boolean;
  name: string;
  hwAddress: string;
  ipv4Enabled: boolean;
  dhcp: boolean;
  address: string;
  prefixLength: number;
  gateway: string;
}

export interface DNSConfig {
  fromDHCP: boolean;
  searchDomain: string[];
  dnsServers: string[];
}

export interface NetworkConfig {
  interfaces: NetworkInterface[];
  dns: DNSConfig;
}

/**
 * Get network interfaces
 */
export async function getNetworkInterfaces(): Promise<NetworkInterface[]> {
  const envelope = createSOAPEnvelope('<tds:GetNetworkInterfaces />');

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get network interfaces');
  }

  const data = parsed.data?.GetNetworkInterfacesResponse as Record<string, unknown> | undefined;
  const interfaces = data?.NetworkInterfaces;

  if (!interfaces) {
    return [];
  }

  // Handle single or array of interfaces
  const interfacesList = Array.isArray(interfaces) ? interfaces : [interfaces];

  return interfacesList.map((iface: Record<string, unknown>) => {
    const info = iface.Info as Record<string, unknown> | undefined;
    const ipv4 = iface.IPv4 as Record<string, unknown> | undefined;
    const config = ipv4?.Config as Record<string, unknown> | undefined;
    const manual = config?.Manual as Record<string, unknown> | undefined;

    return {
      token: String(iface['@_token'] || ''),
      enabled: iface.Enabled === true || iface.Enabled === 'true',
      name: String(info?.Name || 'eth0'),
      hwAddress: String(info?.HwAddress || ''),
      ipv4Enabled: ipv4?.Enabled === true || ipv4?.Enabled === 'true',
      dhcp: config?.DHCP === true || config?.DHCP === 'true',
      address: String(manual?.Address || ''),
      prefixLength: Number(manual?.PrefixLength || 24),
      gateway: '',
    };
  });
}

/**
 * Get DNS configuration
 */
export async function getDNS(): Promise<DNSConfig> {
  const envelope = createSOAPEnvelope('<tds:GetDNS />');

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get DNS configuration');
  }

  const data = parsed.data?.GetDNSResponse as Record<string, unknown> | undefined;
  const dnsInfo = data?.DNSInformation as Record<string, unknown> | undefined;

  const searchDomain = dnsInfo?.SearchDomain;
  const dnsServers = dnsInfo?.DNSManual || dnsInfo?.DNSFromDHCP;

  return {
    fromDHCP: dnsInfo?.FromDHCP === true || dnsInfo?.FromDHCP === 'true',
    searchDomain: Array.isArray(searchDomain)
      ? searchDomain.map(String)
      : searchDomain
        ? [String(searchDomain)]
        : [],
    dnsServers: Array.isArray(dnsServers)
      ? dnsServers.map((d: Record<string, unknown>) => String(d.IPv4Address || ''))
      : dnsServers
        ? [String((dnsServers as Record<string, unknown>).IPv4Address || '')]
        : [],
  };
}

/**
 * Get full network configuration
 */
export async function getNetworkConfig(): Promise<NetworkConfig> {
  const [interfaces, dns] = await Promise.all([getNetworkInterfaces(), getDNS()]);

  return { interfaces, dns };
}

/**
 * Set network interface configuration
 */
export async function setNetworkInterface(
  token: string,
  dhcp: boolean,
  address?: string,
  prefixLength?: number,
): Promise<void> {
  const manualConfig =
    !dhcp && address
      ? `<tt:Manual><tt:Address>${address}</tt:Address><tt:PrefixLength>${prefixLength || 24}</tt:PrefixLength></tt:Manual>`
      : '';

  const body = `<tds:SetNetworkInterfaces>
    <tds:InterfaceToken>${token}</tds:InterfaceToken>
    <tds:NetworkInterface>
      <tt:IPv4>
        <tt:Enabled>true</tt:Enabled>
        <tt:DHCP>${dhcp}</tt:DHCP>
        ${manualConfig}
      </tt:IPv4>
    </tds:NetworkInterface>
  </tds:SetNetworkInterfaces>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to set network interface');
  }
}

/**
 * Set DNS configuration
 */
export async function setDNS(_fromDHCP: boolean, dnsServers?: string[]): Promise<void> {
  const manualDNS =
    !_fromDHCP && dnsServers?.length
      ? dnsServers
          .map(
            (ip) =>
              `<tds:DNSManual><tt:Type>IPv4</tt:Type><tt:IPv4Address>${ip}</tt:IPv4Address></tds:DNSManual>`,
          )
          .join('')
      : '';

  const body = `<tds:SetDNS>
    <tds:FromDHCP>${_fromDHCP}</tds:FromDHCP>
    ${manualDNS}
  </tds:SetDNS>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to set DNS');
  }
}
