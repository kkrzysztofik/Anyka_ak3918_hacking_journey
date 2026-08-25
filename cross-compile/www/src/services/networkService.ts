/**
 * Network Service
 *
 * SOAP operations for network configuration and REST overlay access.
 */
import { ENDPOINTS, authorizedFetch } from '@/services/api';
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

export interface NetworkProtocols {
  http: number;
  rtsp: number;
}

export interface SnmpConfig {
  enabled: boolean;
  port: number;
  community: string;
  sys_contact: string;
  sys_name: string;
  sys_location: string;
}

export type SnmpPatch = Partial<SnmpConfig>;

export interface NetworkConfig {
  interfaces: NetworkInterface[];
  dns: DNSConfig;
  protocols: NetworkProtocols;
}

export interface NetworkOverlayView {
  ssid?: string;
  has_password: boolean;
  security?: string;
  dhcp?: boolean;
  address?: string;
  gateway?: string;
  dns?: string[];
}

export interface NetworkOverlayState {
  pending: NetworkOverlayView;
  has_pending: boolean;
  last_failure: NetworkOverlayView | null;
}

export type NetworkOverlayPatch = Partial<{
  ssid: string;
  password: string;
  security: string;
  dhcp: boolean;
  address: string;
  gateway: string;
  dns: string[];
}>;

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

  const interfacesList = Array.isArray(interfaces) ? interfaces : [interfaces];

  return interfacesList.map((iface: Record<string, unknown>) => {
    const info = iface.Info as Record<string, unknown> | undefined;
    const ipv4 = iface.IPv4 as Record<string, unknown> | undefined;
    const config = ipv4?.Config as Record<string, unknown> | undefined;
    const manual = config?.Manual as Record<string, unknown> | undefined;
    const link = iface.Link as Record<string, unknown> | undefined;
    const operSettings = link?.OperSettings as Record<string, unknown> | undefined;
    const rawSpeed = operSettings?.Speed;
    let parsedSpeed = Number.NaN;
    if (typeof rawSpeed === 'number') {
      parsedSpeed = rawSpeed;
    } else if (typeof rawSpeed === 'string') {
      parsedSpeed = Number(rawSpeed);
    }
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
 * Get default gateway from the device.
 */
export async function getNetworkDefaultGateway(): Promise<string> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    '<tds:GetNetworkDefaultGateway />',
    'GetNetworkDefaultGatewayResponse',
  );

  const gateways = data?.NetworkGateway;
  const gateway = Array.isArray(gateways) ? gateways[0] : gateways;
  const gatewayRecord = gateway as Record<string, unknown> | undefined;
  const ipv4 = gatewayRecord?.IPv4Address;
  if (Array.isArray(ipv4)) {
    return safeString(ipv4[0], '');
  }
  return safeString(ipv4, '');
}

/**
 * Get HTTP and RTSP listener ports.
 */
export async function getNetworkProtocols(): Promise<NetworkProtocols> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    '<tds:GetNetworkProtocols />',
    'GetNetworkProtocolsResponse',
  );

  const protocols = data?.NetworkProtocols;
  let list: unknown[] = [];
  if (Array.isArray(protocols)) {
    list = protocols;
  } else if (protocols) {
    list = [protocols];
  }

  let http = 80;
  let rtsp = 554;

  for (const proto of list) {
    const entry = proto as Record<string, unknown>;
    const name = safeString(entry.Name, '').toUpperCase();
    const ports = entry.Port;
    const portValue = Array.isArray(ports) ? ports[0] : ports;
    const port = Number(portValue);
    if (!Number.isInteger(port) || port < 1 || port > 65535) continue;
    if (name === 'HTTP') http = port;
    if (name === 'RTSP') rtsp = port;
  }

  return { http, rtsp };
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
  const [interfaces, dns, gateway, protocols] = await Promise.all([
    getNetworkInterfaces(),
    getDNS(),
    getNetworkDefaultGateway(),
    getNetworkProtocols(),
  ]);

  if (interfaces[0]) {
    interfaces[0] = { ...interfaces[0], gateway };
  }

  return { interfaces, dns, protocols };
}

/**
 * Read pending overlay state from /api/network.
 */
export async function getNetworkOverlay(): Promise<NetworkOverlayState> {
  const response = await authorizedFetch('/api/network');
  if (!response.ok) {
    throw new Error(`Failed to load network overlay (${response.status})`);
  }
  return (await response.json()) as NetworkOverlayState;
}

/**
 * Write overlay keys via PUT /api/network.
 */
export async function putNetworkOverlay(patch: NetworkOverlayPatch): Promise<void> {
  const body: NetworkOverlayPatch = {};
  for (const [key, value] of Object.entries(patch)) {
    if (value !== undefined) {
      (body as Record<string, unknown>)[key] = value;
    }
  }

  const response = await authorizedFetch('/api/network', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `Overlay save failed (${response.status})`);
  }
}

/**
 * Read SNMP agent settings from GET /api/snmp.
 */
export async function getSnmpConfig(): Promise<SnmpConfig> {
  const response = await authorizedFetch('/api/snmp');
  if (!response.ok) {
    throw new Error(`Failed to load SNMP config (${response.status})`);
  }
  return (await response.json()) as SnmpConfig;
}

/**
 * Write SNMP settings via PUT /api/snmp.
 */
export async function putSnmpConfig(patch: SnmpPatch): Promise<void> {
  const body: SnmpPatch = {};
  for (const [key, value] of Object.entries(patch)) {
    if (value !== undefined) {
      (body as Record<string, unknown>)[key] = value;
    }
  }

  const response = await authorizedFetch('/api/snmp', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `SNMP save failed (${response.status})`);
  }
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
 * Set default gateway
 */
export async function setNetworkDefaultGateway(gateway: string): Promise<void> {
  const body = `<tds:SetNetworkDefaultGateway>
    <tds:NetworkGateway>
      <tt:IPv4Address>${escapeXml(gateway)}</tt:IPv4Address>
    </tds:NetworkGateway>
  </tds:SetNetworkDefaultGateway>`;

  await soapRequest(ENDPOINTS.device, body);
}

/**
 * Set DNS configuration
 */
export async function setDNS(fromDHCP: boolean, dnsServers?: string[]): Promise<void> {
  const manualDNS =
    !fromDHCP && dnsServers?.length
      ? dnsServers
          .map(
            (ip) =>
              `<tds:DNSManual><tt:Type>IPv4</tt:Type><tt:IPv4Address>${escapeXml(ip)}</tt:IPv4Address></tds:DNSManual>`,
          )
          .join('')
      : '';

  const body = `<tds:SetDNS>
    <tds:FromDHCP>${escapeXml(String(fromDHCP))}</tds:FromDHCP>
    ${manualDNS}
  </tds:SetDNS>`;

  await soapRequest(ENDPOINTS.device, body);
}

/**
 * Set HTTP and RTSP listener ports.
 */
export async function setNetworkProtocols(httpPort: number, rtspPort: number): Promise<void> {
  const body = `<tds:SetNetworkProtocols>
    <tds:NetworkProtocols>
      <tt:Name>HTTP</tt:Name>
      <tt:Enabled>true</tt:Enabled>
      <tt:Port>${httpPort}</tt:Port>
    </tds:NetworkProtocols>
    <tds:NetworkProtocols>
      <tt:Name>RTSP</tt:Name>
      <tt:Enabled>true</tt:Enabled>
      <tt:Port>${rtspPort}</tt:Port>
    </tds:NetworkProtocols>
  </tds:SetNetworkProtocols>`;

  await soapRequest(ENDPOINTS.device, body);
}
