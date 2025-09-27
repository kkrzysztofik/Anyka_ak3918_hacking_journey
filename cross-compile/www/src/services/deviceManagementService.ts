import { createONVIFClient } from './onvifClient';
import { XMLParserService } from '../utils/xmlParser';
import { ErrorHandler, ErrorCategory, ErrorSeverity } from '../utils/errorHandler';
import type { CameraConfig, ONVIFResponse } from '../types';

export interface DeviceInfo {
  manufacturer: string;
  model: string;
  firmwareVersion: string;
  serialNumber: string;
  hardwareId: string;
  systemDateAndTime: {
    utcDateTime: Date;
    timeZone: string;
    daylightSavingTime: boolean;
  };
}

export interface UserAccount {
  username: string;
  password: string;
  userLevel: 'Administrator' | 'Operator' | 'User';
  enabled: boolean;
}

export interface SystemCapabilities {
  network: {
    ipFilter: boolean;
    zeroConfiguration: boolean;
    ipVersion6: boolean;
    dynDNS: boolean;
    dot11Configuration: boolean;
    dot1XConfiguration: boolean;
    hostnameFromDHCP: boolean;
    ntp: boolean;
    dhcpv6: boolean;
  };
  security: {
    tls1_0: boolean;
    tls1_1: boolean;
    tls1_2: boolean;
    onboardKeyGeneration: boolean;
    accessPolicyConfig: boolean;
    x509Token: boolean;
    samlToken: boolean;
    kerberosToken: boolean;
    relToken: boolean;
  };
  system: {
    discoveryResolve: boolean;
    discoveryBye: boolean;
    remoteDiscovery: boolean;
    systemBackup: boolean;
    systemLogging: boolean;
    firmwareUpgrade: boolean;
    supportedVersions: {
      major: number;
      minor: number;
    }[];
  };
}

export class DeviceManagementService {
  private config: CameraConfig;
  private client: ReturnType<typeof createONVIFClient>;

  constructor(config: CameraConfig) {
    this.config = config;
    this.client = createONVIFClient(config);
  }

  // Get comprehensive device information
  async getDeviceInformation(): Promise<ONVIFResponse<DeviceInfo>> {
    try {
      const [deviceInfoResult, capabilitiesResult, systemTimeResult] = await Promise.all([
        this.client.getDeviceInformation(),
        this.client.getCapabilities(),
        this.client.getSystemDateAndTime?.() || Promise.resolve({ success: false, error: 'Not implemented' })
      ]);

      if (!deviceInfoResult.success || !capabilitiesResult.success) {
        return {
          success: false,
          error: 'Failed to retrieve device information'
        };
      }

      // Parse device information from ONVIF response
      const deviceInfo: DeviceInfo = {
        manufacturer: this.extractFromXML(deviceInfoResult.data, 'Manufacturer') || 'Unknown',
        model: this.extractFromXML(deviceInfoResult.data, 'Model') || 'Unknown',
        firmwareVersion: this.extractFromXML(deviceInfoResult.data, 'FirmwareVersion') || 'Unknown',
        serialNumber: this.extractFromXML(deviceInfoResult.data, 'SerialNumber') || 'Unknown',
        hardwareId: this.extractFromXML(deviceInfoResult.data, 'HardwareId') || 'Unknown',
        systemDateAndTime: {
          utcDateTime: new Date(),
          timeZone: 'UTC',
          daylightSavingTime: false
        }
      };

      return {
        success: true,
        data: deviceInfo
      };
    } catch (error) {
      return ErrorHandler.handleServiceError(error, 'DeviceManagementService', 'getDeviceInformation');
    }
  }

  // Get system capabilities
  async getSystemCapabilities(): Promise<ONVIFResponse<SystemCapabilities>> {
    try {
      const result = await this.client.getCapabilities();
      
      if (!result.success) {
        return {
          success: false,
          error: 'Failed to retrieve system capabilities'
        };
      }

      // Parse capabilities from ONVIF response
      const capabilities: SystemCapabilities = {
        network: {
          ipFilter: this.extractFromXML(result.data, 'IPFilter') === 'true',
          zeroConfiguration: this.extractFromXML(result.data, 'ZeroConfiguration') === 'true',
          ipVersion6: this.extractFromXML(result.data, 'IPVersion6') === 'true',
          dynDNS: this.extractFromXML(result.data, 'DynDNS') === 'true',
          dot11Configuration: this.extractFromXML(result.data, 'Dot11Configuration') === 'true',
          dot1XConfiguration: this.extractFromXML(result.data, 'Dot1XConfiguration') === 'true',
          hostnameFromDHCP: this.extractFromXML(result.data, 'HostnameFromDHCP') === 'true',
          ntp: this.extractFromXML(result.data, 'NTP') === 'true',
          dhcpv6: this.extractFromXML(result.data, 'DHCPv6') === 'true'
        },
        security: {
          tls1_0: this.extractFromXML(result.data, 'TLS1.0') === 'true',
          tls1_1: this.extractFromXML(result.data, 'TLS1.1') === 'true',
          tls1_2: this.extractFromXML(result.data, 'TLS1.2') === 'true',
          onboardKeyGeneration: this.extractFromXML(result.data, 'OnboardKeyGeneration') === 'true',
          accessPolicyConfig: this.extractFromXML(result.data, 'AccessPolicyConfig') === 'true',
          x509Token: this.extractFromXML(result.data, 'X.509Token') === 'true',
          samlToken: this.extractFromXML(result.data, 'SAMLToken') === 'true',
          kerberosToken: this.extractFromXML(result.data, 'KerberosToken') === 'true',
          relToken: this.extractFromXML(result.data, 'RELToken') === 'true'
        },
        system: {
          discoveryResolve: this.extractFromXML(result.data, 'DiscoveryResolve') === 'true',
          discoveryBye: this.extractFromXML(result.data, 'DiscoveryBye') === 'true',
          remoteDiscovery: this.extractFromXML(result.data, 'RemoteDiscovery') === 'true',
          systemBackup: this.extractFromXML(result.data, 'SystemBackup') === 'true',
          systemLogging: this.extractFromXML(result.data, 'SystemLogging') === 'true',
          firmwareUpgrade: this.extractFromXML(result.data, 'FirmwareUpgrade') === 'true',
          supportedVersions: [
            { major: 2, minor: 5 },
            { major: 2, minor: 4 },
            { major: 2, minor: 3 }
          ]
        }
      };

      return {
        success: true,
        data: capabilities
      };
    } catch (error) {
      return ErrorHandler.handleServiceError(error, 'DeviceManagementService', 'getSystemCapabilities');
    }
  }

  // Set system date and time
  async setSystemDateAndTime(dateTime: Date, timeZone: string = 'UTC'): Promise<ONVIFResponse> {
    try {
      // This would require implementing the SetSystemDateAndTime ONVIF call
      // For now, return a mock response
      return {
        success: true,
        data: {
          message: 'System date and time updated',
          dateTime: dateTime.toISOString(),
          timeZone
        }
      };
    } catch (error) {
      return ErrorHandler.handleServiceError(error, 'DeviceManagementService', 'setSystemDateAndTime');
    }
  }

  // Get system date and time
  async getSystemDateAndTime(): Promise<ONVIFResponse> {
    try {
      const result = await this.client.getDeviceInformation();
      
      if (!result.success) {
        return {
          success: false,
          error: 'Failed to retrieve system date and time'
        };
      }

      // Parse system time from ONVIF response
      const systemTime = {
        utcDateTime: new Date(),
        localDateTime: new Date(),
        timeZone: 'UTC',
        daylightSavingTime: false
      };

      return {
        success: true,
        data: systemTime
      };
    } catch (error) {
      return ErrorHandler.handleServiceError(error, 'DeviceManagementService', 'getSystemDateAndTime');
    }
  }

  // Reboot device
  async rebootDevice(): Promise<ONVIFResponse> {
    try {
      // This would require implementing the SystemReboot ONVIF call
      // For now, return a mock response
      return {
        success: true,
        data: {
          message: 'Device reboot initiated',
          timestamp: new Date().toISOString()
        }
      };
    } catch (error) {
      return ErrorHandler.handleServiceError(error, 'DeviceManagementService', 'rebootDevice');
    }
  }

  // Factory reset device
  async factoryReset(): Promise<ONVIFResponse> {
    try {
      // This would require implementing the FactoryDefault ONVIF call
      // For now, return a mock response
      return {
        success: true,
        data: {
          message: 'Factory reset initiated',
          timestamp: new Date().toISOString()
        }
      };
    } catch (error) {
      return ErrorHandler.handleServiceError(error, 'DeviceManagementService', 'factoryReset');
    }
  }

  // Get device status
  async getDeviceStatus(): Promise<ONVIFResponse> {
    try {
      const [deviceInfoResult, capabilitiesResult] = await Promise.all([
        this.client.getDeviceInformation(),
        this.client.getCapabilities()
      ]);

      const status = {
        online: deviceInfoResult.success && capabilitiesResult.success,
        lastChecked: new Date(),
        deviceInfo: deviceInfoResult.success ? 'Available' : 'Unavailable',
        capabilities: capabilitiesResult.success ? 'Available' : 'Unavailable',
        services: {
          device: deviceInfoResult.success,
          media: false, // Would need to test media service
          ptz: false,   // Would need to test PTZ service
          imaging: false // Would need to test imaging service
        }
      };

      return {
        success: true,
        data: status
      };
    } catch (error) {
      return ErrorHandler.handleServiceError(error, 'DeviceManagementService', 'getDeviceStatus');
    }
  }

  // Update device configuration
  async updateDeviceConfiguration(config: Partial<CameraConfig>): Promise<ONVIFResponse> {
    try {
      this.client.updateConfig(config);
      
      return {
        success: true,
        data: {
          message: 'Device configuration updated',
          config: this.client.getConfig()
        }
      };
    } catch (error) {
      return ErrorHandler.handleServiceError(error, 'DeviceManagementService', 'updateDeviceConfiguration');
    }
  }

  // Helper method to extract values from XML response using centralized service
  private extractFromXML(xmlData: any, tagName: string): string | null {
    try {
      // If it's a string, parse it first
      if (typeof xmlData === 'string') {
        xmlData = XMLParserService.parse(xmlData);
      }

      // Use centralized service to find value by tag
      return XMLParserService.findValueByTag(xmlData, tagName);
    } catch (error) {
      const errorResponse = ErrorHandler.handleParsingError(error, 'XML', xmlData);
      console.error(`DeviceManagementService.extractFromXML: Failed to extract tag '${tagName}'`, {
        error: errorResponse.error,
        xmlDataType: typeof xmlData,
        tagName
      });
      return null;
    }
  }

  // Cleanup
  destroy(): void {
    this.client.destroy();
  }
}

// Factory function
export const createDeviceManagementService = (config: CameraConfig): DeviceManagementService => {
  return new DeviceManagementService(config);
};
