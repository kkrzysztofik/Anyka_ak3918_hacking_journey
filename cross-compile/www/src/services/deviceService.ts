/**
 * Device Service
 *
 * SOAP operations for device management (GetDeviceInformation, GetScopes, SetScopes).
 */
import { ENDPOINTS } from '@/services/api';
import { soapRequest } from '@/services/soap/client';
import { safeString } from '@/utils/safeString';

export interface DeviceInfo {
  manufacturer: string;
  model: string;
  firmwareVersion: string;
  serialNumber: string;
  hardwareId: string;
}

export interface DeviceScope {
  scopeItem: string;
}

export interface DeviceIdentification {
  deviceInfo: DeviceInfo;
  name: string;
  location: string;
}

export async function setDeviceInformation(data: DeviceIdentification): Promise<void> {
  await setScopes(data.name, data.location);
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
 * Get device scopes (name, location, etc.)
 */
export async function getScopes(): Promise<{ name: string; location: string }> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    '<tds:GetScopes />',
    'GetScopesResponse',
  );

  const scopes = data?.Scopes as Array<{ ScopeDef?: string; ScopeItem?: string }> | undefined;

  let name = '';
  let location = '';

  if (scopes && Array.isArray(scopes)) {
    for (const scope of scopes) {
      const item = scope.ScopeItem || '';
      if (item.includes('name/')) {
        name = item.split('name/')[1] || '';
      }
      if (item.includes('location/')) {
        location = item.split('location/')[1] || '';
      }
    }
  }

  return {
    name: decodeURIComponent(name),
    location: decodeURIComponent(location),
  };
}

/**
 * Set device scopes (name, location)
 */
export async function setScopes(name: string, location: string): Promise<void> {
  const encodedName = encodeURIComponent(name);
  const encodedLocation = encodeURIComponent(location);

  const body = `<tds:SetScopes>
    <tds:Scopes>onvif://www.onvif.org/name/${encodedName}</tds:Scopes>
    <tds:Scopes>onvif://www.onvif.org/location/${encodedLocation}</tds:Scopes>
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
    name: scopes.name,
    location: scopes.location,
  };
}
