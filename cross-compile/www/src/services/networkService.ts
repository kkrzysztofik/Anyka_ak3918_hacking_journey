/**
 * Network Service
 *
 * SOAP operations for network configuration.
 */
import { ENDPOINTS } from '@/services/api';
import { escapeXml, soapRequest } from '@/services/soap/client';
import { safeString } from '@/utils/safeString';

/**
 * Safely parse boolean values from API responses.
 * Handles both boolean and string representations.
 */
function parseBoolean(value: unknown): boolean {
  return value === true || value === 'true';
}

export interface NetworkInterface {
  token: string;
  enabled: boolean;
  name: string;
  hwAddress: string;
  /** Link speed in Mbps from ONVIF OperSettings, when reported (> 0). */
  linkSpeedMbps: number | null;
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
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    '<tds:GetNetworkInterfaces />',
    'GetNetworkInterfacesResponse',
  );

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
    const link = iface.Link as Record<string, unknown> | undefined;
    const operSettings = link?.OperSettings as Record<string, unknown> | undefined;
    const rawSpeed = operSettings?.Speed;
    const parsedSpeed =
      typeof rawSpeed === 'number'
        ? rawSpeed
        : typeof rawSpeed === 'string'
          ? Number(rawSpeed)
          : Number.NaN;
    const linkSpeedMbps =
      Number.isFinite(parsedSpeed) && parsedSpeed > 0 ? Math.trunc(parsedSpeed) : null;

    return {
      token: safeString(iface['@_token'], ''),
      enabled: parseBoolean(iface.Enabled),
      name: safeString(info?.Name, 'eth0'),
      hwAddress: safeString(info?.HwAddress, ''),
      linkSpeedMbps,
      ipv4Enabled: parseBoolean(ipv4?.Enabled),
      dhcp: parseBoolean(config?.DHCP),
      address: safeString(manual?.Address, ''),
      prefixLength: Number(manual?.PrefixLength || 24),
      gateway: '',
    };
  });
}

/**
 * Get DNS configuration
 */
export async function getDNS(): Promise<DNSConfig> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    '<tds:GetDNS />',
    'GetDNSResponse',
  );

  const dnsInfo = data?.DNSInformation as Record<string, unknown> | undefined;

  const searchDomain = dnsInfo?.SearchDomain;
  const dnsServers = dnsInfo?.DNSManual || dnsInfo?.DNSFromDHCP;

  let searchDomainList: string[] = [];
  if (Array.isArray(searchDomain)) {
    searchDomainList = searchDomain.map((item) => safeString(item, ''));
  } else if (searchDomain) {
    searchDomainList = [safeString(searchDomain, '')];
  }

  let dnsServersList: string[] = [];
  if (Array.isArray(dnsServers)) {
    dnsServersList = dnsServers.map((d: Record<string, unknown>) => safeString(d.IPv4Address, ''));
  } else if (dnsServers) {
    dnsServersList = [safeString((dnsServers as Record<string, unknown>).IPv4Address, '')];
  }

  return {
    fromDHCP: parseBoolean(dnsInfo?.FromDHCP),
    searchDomain: searchDomainList,
    dnsServers: dnsServersList,
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
  const escapedToken = escapeXml(token);
  const escapedAddress = address ? escapeXml(address) : undefined;

  const manualConfig =
    !dhcp && escapedAddress
      ? `<tt:Manual><tt:Address>${escapedAddress}</tt:Address><tt:PrefixLength>${prefixLength || 24}</tt:PrefixLength></tt:Manual>`
      : '';

  const body = `<tds:SetNetworkInterfaces>
    <tds:InterfaceToken>${escapedToken}</tds:InterfaceToken>
    <tds:NetworkInterface>
      <tt:IPv4>
        <tt:Enabled>true</tt:Enabled>
        <tt:DHCP>${escapeXml(String(dhcp))}</tt:DHCP>
        ${manualConfig}
      </tt:IPv4>
    </tds:NetworkInterface>
  </tds:SetNetworkInterfaces>`;

  await soapRequest(ENDPOINTS.device, body);
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
              `<tds:DNSManual><tt:Type>IPv4</tt:Type><tt:IPv4Address>${escapeXml(ip)}</tt:IPv4Address></tds:DNSManual>`,
          )
          .join('')
      : '';

  const body = `<tds:SetDNS>
    <tds:FromDHCP>${escapeXml(String(_fromDHCP))}</tds:FromDHCP>
    ${manualDNS}
  </tds:SetDNS>`;

  await soapRequest(ENDPOINTS.device, body);
}
