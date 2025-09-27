import axios, { AxiosResponse } from 'axios';
import { XMLParserService } from '../utils/xmlParser';
import { ValidationUtils } from '../utils/validation';
import { ErrorHandler, ErrorCategory, ErrorSeverity } from '../utils/errorHandler';
import type { 
  CameraConfig, 
  PTZDirection, 
  ImagingSettings, 
  ONVIFResponse,
  Endpoint 
} from '../types';

// ONVIF Client using custom implementation with fast-xml-parser
export class ONVIFClient {
  private config: CameraConfig;
  private baseUrl: string;

  constructor(config: CameraConfig) {
    this.config = config;
    this.baseUrl = `http://${config.ip}:${config.onvifPort}`;
  }

  // Update camera configuration
  updateConfig(config: Partial<CameraConfig>): void {
    // Validate the configuration before updating
    const validation = ValidationUtils.validateCameraConfig({ ...this.config, ...config });
    if (!validation.isValid) {
      throw new Error(`Invalid camera configuration: ${validation.error}`);
    }
    
    this.config = { ...this.config, ...config };
    this.baseUrl = `http://${this.config.ip}:${this.config.onvifPort}`;
  }

  // Send SOAP request to ONVIF service
  private async sendSOAPRequest(service: string, action: string, body: any): Promise<ONVIFResponse> {
    try {
      const url = `${this.baseUrl}/onvif/${service}`;
      
      // Build SOAP envelope using centralized service
      const soapEnvelope = XMLParserService.createSOAPEnvelope(body);
      const soapXml = XMLParserService.build(soapEnvelope);

      const response: AxiosResponse<string> = await axios.post(url, soapXml, {
        headers: {
          'Content-Type': 'application/soap+xml; charset=utf-8',
          'SOAPAction': `"${action}"`
        },
        timeout: 10000
      });

      // Parse XML response using centralized service
      const parsedResponse = XMLParserService.parse(response.data);
      
      // Validate SOAP response structure
      if (!XMLParserService.validateSOAPResponse(parsedResponse)) {
        console.warn('ONVIFClient.sendSOAPRequest: Invalid SOAP response structure');
      }
      
      return { 
        success: true, 
        data: parsedResponse 
      };
    } catch (error) {
      const errorResponse = ErrorHandler.handleNetworkError(error, `${this.baseUrl}/onvif/${service}`, 'POST');
      return {
        success: false,
        error: errorResponse.error
      };
    }
  }

  // Extract value from XML response using path
  private extractValue(data: any, path: string): any {
    return XMLParserService.extractValue(data, path);
  }

  // Device Information
  async getDeviceInformation(): Promise<ONVIFResponse> {
    try {
      const body = {
        'tds:GetDeviceInformation': {}
      };

      const result = await this.sendSOAPRequest(
        'device_service',
        'http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation',
        body
      );

      if (result.success) {
        // Extract device information from SOAP response
        const deviceInfo = {
          manufacturer: this.extractValue(result.data, 'soap:Envelope.soap:Body.tds:GetDeviceInformationResponse.tds:Manufacturer'),
          model: this.extractValue(result.data, 'soap:Envelope.soap:Body.tds:GetDeviceInformationResponse.tds:Model'),
          firmwareVersion: this.extractValue(result.data, 'soap:Envelope.soap:Body.tds:GetDeviceInformationResponse.tds:FirmwareVersion'),
          serialNumber: this.extractValue(result.data, 'soap:Envelope.soap:Body.tds:GetDeviceInformationResponse.tds:SerialNumber'),
          hardwareId: this.extractValue(result.data, 'soap:Envelope.soap:Body.tds:GetDeviceInformationResponse.tds:HardwareId')
        };

        return { success: true, data: deviceInfo };
      }

      return result;
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  // Get Capabilities
  async getCapabilities(): Promise<ONVIFResponse> {
    try {
      const body = {
        'tds:GetCapabilities': {
          'tds:Category': ['All']
        }
      };

      const result = await this.sendSOAPRequest(
        'device_service',
        'http://www.onvif.org/ver10/device/wsdl/GetCapabilities',
        body
      );

      if (result.success) {
        // Extract capabilities from SOAP response
        const capabilities = this.extractValue(result.data, 'soap:Envelope.soap:Body.tds:GetCapabilitiesResponse.tds:Capabilities');
        
        return { success: true, data: capabilities || {} };
      }

      return result;
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  // Get System Date and Time
  async getSystemDateAndTime(): Promise<ONVIFResponse> {
    try {
      const body = {
        'tds:GetSystemDateAndTime': {}
      };

      const result = await this.sendSOAPRequest(
        'device_service',
        'http://www.onvif.org/ver10/device/wsdl/GetSystemDateAndTime',
        body
      );

      if (result.success) {
        const systemTime = this.extractValue(result.data, 'soap:Envelope.soap:Body.tds:GetSystemDateAndTimeResponse.tds:SystemDateAndTime');
        
        return { success: true, data: systemTime || {} };
      }

      return result;
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  // PTZ Operations
  async stopPTZ(profileToken?: string): Promise<ONVIFResponse> {
    try {
      const body = {
        'tptz:Stop': {
          'tptz:ProfileToken': profileToken || 'MainProfile',
          'tptz:PanTilt': true,
          'tptz:Zoom': true
        }
      };

      return await this.sendSOAPRequest(
        'ptz_service',
        'http://www.onvif.org/ver20/ptz/wsdl/Stop',
        body
      );
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  async movePTZ(direction: string, profileToken?: string): Promise<ONVIFResponse> {
    try {
      // Validate direction input
      const directionValidation = ValidationUtils.validatePTZDirection(direction);
      if (!directionValidation.isValid) {
        return {
          success: false,
          error: directionValidation.error
        };
      }

      // Validate profile token if provided
      if (profileToken) {
        const tokenValidation = ValidationUtils.validateProfileToken(profileToken);
        if (!tokenValidation.isValid) {
          return {
            success: false,
            error: tokenValidation.error
          };
        }
      }

      const speeds: Record<string, PTZDirection> = {
        'up': { x: 0, y: 0.5 },
        'down': { x: 0, y: -0.5 },
        'left': { x: -0.5, y: 0 },
        'right': { x: 0.5, y: 0 },
        'left_up': { x: -0.5, y: 0.5 },
        'right_up': { x: 0.5, y: 0.5 },
        'left_down': { x: -0.5, y: -0.5 },
        'right_down': { x: 0.5, y: -0.5 }
      };

      const speed = speeds[direction];

      const body = {
        'tptz:RelativeMove': {
          'tptz:ProfileToken': profileToken || 'MainProfile',
          'tptz:Translation': {
            'tt:PanTilt': {
              '@_x': speed.x,
              '@_y': speed.y
            }
          },
          'tptz:Speed': {
            'tt:PanTilt': {
              '@_x': speed.x,
              '@_y': speed.y
            }
          }
        }
      };

      return await this.sendSOAPRequest(
        'ptz_service',
        'http://www.onvif.org/ver20/ptz/wsdl/RelativeMove',
        body
      );
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  // Get PTZ Configuration
  async getPTZConfiguration(profileToken?: string): Promise<ONVIFResponse> {
    try {
      const body = {
        'tptz:GetConfiguration': {
          'tptz:PTZConfigurationToken': profileToken || 'MainProfile'
        }
      };

      const result = await this.sendSOAPRequest(
        'ptz_service',
        'http://www.onvif.org/ver20/ptz/wsdl/GetConfiguration',
        body
      );

      if (result.success) {
        const ptzConfig = this.extractValue(result.data, 'soap:Envelope.soap:Body.tptz:GetConfigurationResponse.tptz:PTZConfiguration');
        
        return { success: true, data: ptzConfig || {} };
      }

      return result;
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  // Imaging Operations
  async getImagingSettings(videoSourceToken?: string): Promise<ONVIFResponse> {
    try {
      const body = {
        'timg:GetImagingSettings': {
          'timg:VideoSourceToken': videoSourceToken || 'VideoSourceToken'
        }
      };

      const result = await this.sendSOAPRequest(
        'imaging_service',
        'http://www.onvif.org/ver20/imaging/wsdl/GetImagingSettings',
        body
      );

      if (result.success) {
        const imagingSettings = this.extractValue(result.data, 'soap:Envelope.soap:Body.timg:GetImagingSettingsResponse.timg:ImagingSettings');
        
        return { success: true, data: imagingSettings || {} };
      }

      return result;
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  async setImagingSettings(settings: ImagingSettings, videoSourceToken?: string): Promise<ONVIFResponse> {
    try {
      // Validate imaging settings
      const settingsValidation = ValidationUtils.validateImagingSettings(settings);
      if (!settingsValidation.isValid) {
        return {
          success: false,
          error: settingsValidation.error
        };
      }

      // Validate video source token if provided
      if (videoSourceToken) {
        const tokenValidation = ValidationUtils.validateProfileToken(videoSourceToken);
        if (!tokenValidation.isValid) {
          return {
            success: false,
            error: tokenValidation.error
          };
        }
      }

      const { brightness, contrast, saturation } = settings;
      
      const body = {
        'timg:SetImagingSettings': {
          'timg:VideoSourceToken': videoSourceToken || 'VideoSourceToken',
          'timg:ImagingSettings': {
            'tt:Brightness': brightness,
            'tt:Contrast': contrast,
            'tt:ColorSaturation': saturation
          }
        }
      };

      return await this.sendSOAPRequest(
        'imaging_service',
        'http://www.onvif.org/ver20/imaging/wsdl/SetImagingSettings',
        body
      );
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  // Media Operations
  async getProfiles(): Promise<ONVIFResponse> {
    try {
      const body = {
        'trt:GetProfiles': {}
      };

      const result = await this.sendSOAPRequest(
        'media_service',
        'http://www.onvif.org/ver10/media/wsdl/GetProfiles',
        body
      );

      if (result.success) {
        const profiles = this.extractValue(result.data, 'soap:Envelope.soap:Body.trt:GetProfilesResponse.trt:Profiles');
        
        return { success: true, data: profiles || {} };
      }

      return result;
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  async getStreamUri(profileToken?: string): Promise<ONVIFResponse> {
    try {
      const body = {
        'trt:GetStreamUri': {
          'trt:ProfileToken': profileToken || 'MainProfile',
          'trt:StreamSetup': {
            'tt:Stream': 'RTP-Unicast',
            'tt:Transport': {
              'tt:Protocol': 'RTSP'
            }
          }
        }
      };

      const result = await this.sendSOAPRequest(
        'media_service',
        'http://www.onvif.org/ver10/media/wsdl/GetStreamUri',
        body
      );

      if (result.success) {
        const streamUri = this.extractValue(result.data, 'soap:Envelope.soap:Body.trt:GetStreamUriResponse.trt:Uri');
        
        return { success: true, data: { uri: streamUri } };
      }

      return result;
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  // Get snapshot URI
  async getSnapshotUri(profileToken?: string): Promise<ONVIFResponse> {
    try {
      const body = {
        'trt:GetSnapshotUri': {
          'trt:ProfileToken': profileToken || 'MainProfile'
        }
      };

      const result = await this.sendSOAPRequest(
        'media_service',
        'http://www.onvif.org/ver10/media/wsdl/GetSnapshotUri',
        body
      );

      if (result.success) {
        const snapshotUri = this.extractValue(result.data, 'soap:Envelope.soap:Body.trt:GetSnapshotUriResponse.trt:Uri');
        
        return { success: true, data: { uri: snapshotUri } };
      }

      return result;
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  // Get presets
  async getPresets(profileToken?: string): Promise<ONVIFResponse> {
    try {
      const body = {
        'tptz:GetPresets': {
          'tptz:ProfileToken': profileToken || 'MainProfile'
        }
      };

      const result = await this.sendSOAPRequest(
        'ptz_service',
        'http://www.onvif.org/ver20/ptz/wsdl/GetPresets',
        body
      );

      if (result.success) {
        const presets = this.extractValue(result.data, 'soap:Envelope.soap:Body.tptz:GetPresetsResponse.tptz:Preset');
        
        return { success: true, data: { preset: presets || [] } };
      }

      return result;
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  // Go to preset
  async gotoPreset(presetToken: string, profileToken?: string): Promise<ONVIFResponse> {
    try {
      // Validate preset token
      const presetValidation = ValidationUtils.validateProfileToken(presetToken);
      if (!presetValidation.isValid) {
        return {
          success: false,
          error: presetValidation.error
        };
      }

      // Validate profile token if provided
      if (profileToken) {
        const tokenValidation = ValidationUtils.validateProfileToken(profileToken);
        if (!tokenValidation.isValid) {
          return {
            success: false,
            error: tokenValidation.error
          };
        }
      }

      const body = {
        'tptz:GotoPreset': {
          'tptz:ProfileToken': profileToken || 'MainProfile',
          'tptz:PresetToken': presetToken
        }
      };

      return await this.sendSOAPRequest(
        'ptz_service',
        'http://www.onvif.org/ver20/ptz/wsdl/GotoPreset',
        body
      );
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  // Set preset
  async setPreset(presetName: string, profileToken?: string): Promise<ONVIFResponse> {
    try {
      // Validate preset name
      const nameValidation = ValidationUtils.validatePresetName(presetName);
      if (!nameValidation.isValid) {
        return {
          success: false,
          error: nameValidation.error
        };
      }

      // Validate profile token if provided
      if (profileToken) {
        const tokenValidation = ValidationUtils.validateProfileToken(profileToken);
        if (!tokenValidation.isValid) {
          return {
            success: false,
            error: tokenValidation.error
          };
        }
      }

      const body = {
        'tptz:SetPreset': {
          'tptz:ProfileToken': profileToken || 'MainProfile',
          'tptz:PresetName': presetName
        }
      };

      const result = await this.sendSOAPRequest(
        'ptz_service',
        'http://www.onvif.org/ver20/ptz/wsdl/SetPreset',
        body
      );

      if (result.success) {
        const presetToken = this.extractValue(result.data, 'soap:Envelope.soap:Body.tptz:SetPresetResponse.tptz:PresetToken');
        
        return { 
          success: true, 
          data: { 
            presetToken,
            presetName 
          } 
        };
      }

      return result;
    } catch (error) {
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      };
    }
  }

  // Get endpoints for this camera
  getEndpoints(): Endpoint[] {
    return [
      { 
        label: 'RTSP Main', 
        url: `rtsp://${this.config.ip}:${this.config.rtspPort}/vs0`,
        type: 'rtsp'
      },
      { 
        label: 'RTSP Sub', 
        url: `rtsp://${this.config.ip}:${this.config.rtspPort}/vs1`,
        type: 'rtsp'
      },
      { 
        label: 'Snapshot', 
        url: `http://${this.config.ip}:${this.config.snapshotPort}/snapshot.jpeg`,
        type: 'http'
      },
      { 
        label: 'ONVIF Device', 
        url: `http://${this.config.ip}:${this.config.onvifPort}/onvif/device_service`,
        type: 'onvif'
      },
      { 
        label: 'ONVIF PTZ', 
        url: `http://${this.config.ip}:${this.config.onvifPort}/onvif/ptz_service`,
        type: 'onvif'
      },
      { 
        label: 'ONVIF Media', 
        url: `http://${this.config.ip}:${this.config.onvifPort}/onvif/media_service`,
        type: 'onvif'
      },
    ];
  }

  // Get current configuration
  getConfig(): CameraConfig {
    return { ...this.config };
  }

  // Cleanup
  destroy(): void {
    // No cleanup needed for custom implementation
  }
}

// Factory function to create ONVIF client
export const createONVIFClient = (config: CameraConfig): ONVIFClient => {
  return new ONVIFClient(config);
};