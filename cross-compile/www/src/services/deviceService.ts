/**
 * Device Service
 *
 * SOAP operations for device management (GetDeviceInformation, GetScopes, SetScopes).
 */
import { ENDPOINTS, apiClient } from '@/services/api';
import { createSOAPEnvelope, parseSOAPResponse, soapBodies } from '@/services/soap/client';

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
  const envelope = createSOAPEnvelope(soapBodies.getDeviceInformation());

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get device information');
  }

  const data = parsed.data?.GetDeviceInformationResponse as Record<string, unknown> | undefined;
  if (!data) {
    throw new Error('Invalid response: missing GetDeviceInformationResponse');
  }

  // Helper to safely convert to string, avoiding object stringification
  const safeString = (value: unknown, defaultValue: string): string => {
    if (value === null || value === undefined) {
      return defaultValue;
    }
    if (typeof value === 'string') {
      return value || defaultValue;
    }
    if (typeof value === 'object') {
      return defaultValue;
    }
    return String(value);
  };

  return {
    manufacturer: safeString(data.Manufacturer, 'Unknown'),
    model: safeString(data.Model, 'Unknown'),
    firmwareVersion: safeString(data.FirmwareVersion, 'Unknown'),
    serialNumber: safeString(data.SerialNumber, 'Unknown'),
    hardwareId: safeString(data.HardwareId, 'Unknown'),
  };
}

/**
 * Get device scopes (name, location, etc.)
 */
export async function getScopes(): Promise<{ name: string; location: string }> {
  const envelope = createSOAPEnvelope('<tds:GetScopes />');

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get scopes');
  }

  const data = parsed.data?.GetScopesResponse as Record<string, unknown> | undefined;
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

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to set scopes');
  }
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
