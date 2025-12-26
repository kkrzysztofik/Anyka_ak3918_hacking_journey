/**
 * Network Service
 *
 * SOAP operations for network configuration.
 */
import { ENDPOINTS } from '@/services/api';
import { soapRequest } from '@/services/soap/client';
import { safeString } from '@/utils/safeString';

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

    return {
      token: safeString(iface['@_token'], ''),
      enabled: iface.Enabled === true || iface.Enabled === 'true',
      name: safeString(info?.Name, 'eth0'),
      hwAddress: safeString(info?.HwAddress, ''),
      ipv4Enabled: ipv4?.Enabled === true || ipv4?.Enabled === 'true',
      dhcp: config?.DHCP === true || config?.DHCP === 'true',
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
    fromDHCP: dnsInfo?.FromDHCP === true || dnsInfo?.FromDHCP === 'true',
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
              `<tds:DNSManual><tt:Type>IPv4</tt:Type><tt:IPv4Address>${ip}</tt:IPv4Address></tds:DNSManual>`,
          )
          .join('')
      : '';

  const body = `<tds:SetDNS>
    <tds:FromDHCP>${_fromDHCP}</tds:FromDHCP>
    ${manualDNS}
  </tds:SetDNS>`;

  await soapRequest(ENDPOINTS.device, body);
}
