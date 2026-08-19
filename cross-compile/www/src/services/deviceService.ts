/**
 * Device Service
 *
 * SOAP operations for device management (GetDeviceInformation, GetScopes, SetScopes).
 */
import { ENDPOINTS } from '@/services/api';
import { escapeXml, soapRequest } from '@/services/soap/client';
import { safeString } from '@/utils/safeString';

export interface DeviceInfo {
  manufacturer: string;
  model: string;
  firmwareVersion: string;
  serialNumber: string;
  hardwareId: string;
}

export interface Scope {
  scopeDef: 'Fixed' | 'Configurable';
  scopeItem: string;
}

export interface DeviceIdentification {
  deviceInfo: DeviceInfo;
  name: string;
  location: string;
}

export type DiscoveryMode = 'Discoverable' | 'NonDiscoverable';

const NAME_PREFIX = 'onvif://www.onvif.org/name/';
const LOCATION_PREFIX = 'onvif://www.onvif.org/location/';

function remainderAfter(scopeItem: string, prefix: string): string | undefined {
  if (!scopeItem.startsWith(prefix)) {
    return undefined;
  }
  return scopeItem.slice(prefix.length);
}

/** Form-managed name/location scopes are a single path segment after the prefix. */
function isManagedScope(scopeItem: string, prefix: string): boolean {
  const rest = remainderAfter(scopeItem, prefix);
  return rest !== undefined && !rest.includes('/');
}

function asScopeElements(scopes: unknown): Array<{ ScopeDef?: string; ScopeItem?: string }> {
  if (scopes === undefined || scopes === null) {
    return [];
  }
  return Array.isArray(scopes) ? scopes : [scopes];
}

function decodeScopeValue(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

export function nameFromScopes(scopes: Scope[]): string {
  for (const scope of scopes) {
    const rest = remainderAfter(scope.scopeItem, NAME_PREFIX);
    if (rest !== undefined) {
      return decodeScopeValue(rest);
    }
  }
  return '';
}

export function locationFromScopes(scopes: Scope[]): string {
  for (const scope of scopes) {
    const rest = remainderAfter(scope.scopeItem, LOCATION_PREFIX);
    if (rest !== undefined && !rest.includes('/')) {
      return decodeScopeValue(rest);
    }
  }
  return '';
}

export function scopesForSave(
  scopes: Scope[],
  values: { name: string; location: string },
): string[] {
  const kept = scopes
    .filter((scope) => scope.scopeDef === 'Configurable')
    .map((scope) => scope.scopeItem)
    .filter((item) => !isManagedScope(item, NAME_PREFIX) && !isManagedScope(item, LOCATION_PREFIX));

  return [
    ...kept,
    `${NAME_PREFIX}${encodeURIComponent(values.name)}`,
    `${LOCATION_PREFIX}${encodeURIComponent(values.location)}`,
  ];
}

/**
 * Get device information (manufacturer, model, firmware, serial, hardware ID)
 */
export async function getDeviceInformation(): Promise<DeviceInfo> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    '<tds:GetDeviceInformation />',
    'GetDeviceInformationResponse',
  );

  if (!data) {
    throw new Error('Invalid response: missing GetDeviceInformationResponse');
  }

  return {
    manufacturer: safeString(data?.Manufacturer, 'Unknown'),
    model: safeString(data?.Model, 'Unknown'),
    firmwareVersion: safeString(data?.FirmwareVersion, 'Unknown'),
    serialNumber: safeString(data?.SerialNumber, 'Unknown'),
    hardwareId: safeString(data?.HardwareId, 'Unknown'),
  };
}

/**
 * Get the device's ONVIF scopes, including Fixed entries.
 */
export async function getScopes(): Promise<Scope[]> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    '<tds:GetScopes />',
    'GetScopesResponse',
  );

  return asScopeElements(data?.Scopes)
    .map((scope) => {
      const scopeDef = scope.ScopeDef === 'Fixed' ? 'Fixed' : 'Configurable';
      return {
        scopeDef,
        scopeItem: safeString(scope.ScopeItem, ''),
      } satisfies Scope;
    })
    .filter((scope) => scope.scopeItem.length > 0);
}

/**
 * Replace the configurable scope list. Fixed scopes are never sent.
 */
export async function setScopes(scopeItems: string[]): Promise<void> {
  const body = `<tds:SetScopes>
    ${scopeItems.map((item) => `<tds:Scopes>${escapeXml(item)}</tds:Scopes>`).join('\n    ')}
  </tds:SetScopes>`;

  await soapRequest(ENDPOINTS.device, body);
}

/**
 * Get complete device identification (info + scopes)
 */
export async function getDeviceIdentification(): Promise<DeviceIdentification> {
  const [deviceInfo, scopes] = await Promise.all([getDeviceInformation(), getScopes()]);

  return {
    deviceInfo,
    name: nameFromScopes(scopes),
    location: locationFromScopes(scopes),
  };
}

export async function getDiscoveryMode(): Promise<DiscoveryMode> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    '<tds:GetDiscoveryMode />',
    'GetDiscoveryModeResponse',
  );
  const mode = safeString(data?.DiscoveryMode, 'Discoverable');
  return mode === 'NonDiscoverable' ? 'NonDiscoverable' : 'Discoverable';
}

export async function setDiscoveryMode(mode: DiscoveryMode): Promise<void> {
  await soapRequest(
    ENDPOINTS.device,
    `<tds:SetDiscoveryMode><tds:DiscoveryMode>${mode}</tds:DiscoveryMode></tds:SetDiscoveryMode>`,
  );
}

export async function getHostname(): Promise<string> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    '<tds:GetHostname />',
    'GetHostnameResponse',
  );
  const info = data?.HostnameInformation as { Name?: unknown } | undefined;
  return safeString(info?.Name, '');
}

export async function setHostname(name: string): Promise<void> {
  await soapRequest(
    ENDPOINTS.device,
    `<tds:SetHostname><tds:Name>${escapeXml(name)}</tds:Name></tds:SetHostname>`,
  );
}
