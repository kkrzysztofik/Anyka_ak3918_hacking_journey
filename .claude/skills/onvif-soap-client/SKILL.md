---
name: onvif-soap-client
description: |
  SOAP client for interacting with ONVIF camera services from the WebUI.
  Triggers on: "SOAP client", "parse XML", "service method", "ONVIF request", "camera API", "fetch device".
version: 1.0.0
---

# ONVIF SOAP Client Implementation

Build TypeScript/React SOAP clients for consuming ONVIF camera services. Handle SOAP request construction, XML parsing, authentication, and error handling.

## SOAP Basics

SOAP (Simple Object Access Protocol) uses XML-formatted messages over HTTP for Remote Procedure Calls (RPC). ONVIF services respond with SOAP envelopes containing either data or fault messages.

### SOAP Request Structure

```xml
<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
    <s:Header>
        <!-- Authentication headers go here -->
    </s:Header>
    <s:Body>
        <tds:GetDeviceInformation/>
    </s:Body>
</s:Envelope>
```

### SOAP Response Structure

```xml
<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tds="http://www.onvif.org/ver10/device/wsdl"
            xmlns:tt="http://www.onvif.org/ver10/schema">
    <s:Body>
        <tds:GetDeviceInformationResponse>
            <tds:DeviceInformation>
                <tt:Manufacturer>Anyka</tt:Manufacturer>
                <tt:Model>AK3918</tt:Model>
                <tt:FirmwareVersion>1.0.0</tt:FirmwareVersion>
                <tt:SerialNumber>SN123456</tt:SerialNumber>
                <tt:HardwareId>HW001</tt:HardwareId>
            </tds:DeviceInformation>
        </tds:GetDeviceInformationResponse>
    </s:Body>
</s:Envelope>
```

## SOAP Client Builder

Create a reusable SOAP client service:

```typescript
// services/soap/client.ts

interface SoapClientConfig {
  baseUrl: string;  // e.g., "http://192.168.1.100:8080"
  username?: string;
  password?: string;
}

interface SoapRequest {
  service: string;  // e.g., "device_service"
  action: string;   // e.g., "GetDeviceInformation"
  body: string;     // XML content
}

interface SoapResponse<T> {
  success: boolean;
  data?: T;
  error?: SoapFault;
}

interface SoapFault {
  code: string;
  subcode: string;
  reason: string;
}

export class SoapClient {
  private config: SoapClientConfig;
  private nonce: string = '';
  private timestamp: string = '';

  constructor(config: SoapClientConfig) {
    this.config = config;
  }

  /**
   * Make SOAP request to camera service
   */
  async request<T>(
    req: SoapRequest,
    retries: number = 0
  ): Promise<SoapResponse<T>> {
    try {
      const envelope = this.buildEnvelope(req);
      const url = `${this.config.baseUrl}/onvif/${req.service}`;

      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'text/xml; charset=utf-8',
          'SOAPAction': req.action,
        },
        body: envelope,
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const responseText = await response.text();
      return this.parseResponse<T>(responseText);
    } catch (error) {
      // Retry on network errors
      if (retries < 3 && error instanceof TypeError) {
        await new Promise((r) => setTimeout(r, 1000 * (retries + 1)));
        return this.request<T>(req, retries + 1);
      }

      return {
        success: false,
        error: {
          code: 'ConnectionError',
          subcode: 'NetworkError',
          reason: error instanceof Error ? error.message : 'Unknown error',
        },
      };
    }
  }

  /**
   * Build SOAP envelope with optional authentication
   */
  private buildEnvelope(req: SoapRequest): string {
    const ns = {
      s: 'http://www.w3.org/2003/05/soap-envelope',
      wsu: 'http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd',
      wsse: 'http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd',
    };

    let header = '';

    // Add WS-Security header if credentials provided
    if (this.config.username && this.config.password) {
      header = this.buildSecurityHeader();
    }

    return `<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="${ns.s}">
    ${header ? `<s:Header>${header}</s:Header>` : ''}
    <s:Body>${req.body}</s:Body>
</s:Envelope>`;
  }

  /**
   * Build WS-Security header for authentication
   */
  private buildSecurityHeader(): string {
    this.nonce = this.generateNonce();
    this.timestamp = new Date().toISOString();

    const passwordDigest = this.calculatePasswordDigest(
      this.nonce,
      this.timestamp,
      this.config.password || ''
    );

    return `<wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd"
        xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
        <wsu:Timestamp wsu:Id="Timestamp-1">
            <wsu:Created>${this.timestamp}</wsu:Created>
            <wsu:Expires>${new Date(Date.now() + 3600000).toISOString()}</wsu:Expires>
        </wu:Timestamp>
        <wsse:UsernameToken wsu:Id="UsernameToken-1">
            <wsse:Username>${this.config.username}</wsse:Username>
            <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">
                ${passwordDigest}
            </wsse:Password>
            <wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">
                ${this.nonce}
            </wsse:Nonce>
            <wsu:Created>${this.timestamp}</wsu:Created>
        </wsse:UsernameToken>
    </wsse:Security>`;
  }

  /**
   * Generate random nonce
   */
  private generateNonce(): string {
    const bytes = new Uint8Array(16);
    crypto.getRandomValues(bytes);
    return btoa(String.fromCharCode(...bytes));
  }

  /**
   * Calculate password digest per WS-Security spec
   * Digest = Base64( SHA1( Base64Decode(Nonce) + Created + Password ) )
   */
  private calculatePasswordDigest(
    nonce: string,
    created: string,
    password: string
  ): string {
    // Note: This is simplified. In production, use proper crypto libraries
    // For now, use HTTP Digest as fallback
    return btoa(password);
  }

  /**
   * Parse SOAP response and extract data or fault
   */
  private parseResponse<T>(responseText: string): SoapResponse<T> {
    try {
      const parser = new DOMParser();
      const doc = parser.parseFromString(responseText, 'text/xml');

      // Check for parse errors
      if (doc.getElementsByTagName('parsererror').length > 0) {
        return {
          success: false,
          error: {
            code: 'ParseError',
            subcode: 'InvalidXML',
            reason: 'Failed to parse SOAP response',
          },
        };
      }

      // Check for SOAP Fault
      const faultElement = doc.querySelector('s\\:Fault, Fault');
      if (faultElement) {
        return this.extractFault(faultElement);
      }

      // Extract response data
      const bodyElement = doc.querySelector('s\\:Body, Body');
      if (!bodyElement) {
        return {
          success: false,
          error: {
            code: 'InvalidResponse',
            subcode: 'NoBody',
            reason: 'SOAP response missing Body element',
          },
        };
      }

      // Parse response content
      const data = this.extractResponseData<T>(bodyElement);

      return {
        success: true,
        data,
      };
    } catch (error) {
      return {
        success: false,
        error: {
          code: 'ParseError',
          subcode: 'Exception',
          reason: error instanceof Error ? error.message : 'Unknown error',
        },
      };
    }
  }

  /**
   * Extract SOAP Fault information
   */
  private extractFault(faultElement: Element): SoapResponse<never> {
    const codeElement = faultElement.querySelector('Code, Code Value');
    const reasonElement = faultElement.querySelector('Reason, Reason Text');

    return {
      success: false,
      error: {
        code: codeElement?.textContent || 'Unknown',
        subcode:
          faultElement.querySelector('Subcode, Subcode Value')?.textContent || '',
        reason: reasonElement?.textContent || 'Unknown fault',
      },
    };
  }

  /**
   * Extract typed response data from SOAP body
   */
  private extractResponseData<T>(bodyElement: Element): T {
    // This is a simplified extraction. In production, use XML-to-JSON library
    // or define specific extractors for each service response
    const result: any = {};

    // Extract all child elements
    Array.from(bodyElement.children).forEach((child) => {
      if (child.nodeType === Node.ELEMENT_NODE) {
        result[child.localName] = this.extractElement(child);
      }
    });

    return result as T;
  }

  /**
   * Recursively extract element data
   */
  private extractElement(element: Element): any {
    if (element.children.length === 0) {
      return element.textContent;
    }

    const result: any = {};
    Array.from(element.children).forEach((child) => {
      const key = child.localName;
      const value = this.extractElement(child);

      if (result[key]) {
        // Handle multiple elements with same name
        if (!Array.isArray(result[key])) {
          result[key] = [result[key]];
        }
        result[key].push(value);
      } else {
        result[key] = value;
      }
    });

    return result;
  }
}
```

## Service-Specific Clients

Create typed clients for each ONVIF service:

```typescript
// services/soap/device-service.ts

export interface DeviceInfo {
  manufacturer: string;
  model: string;
  firmwareVersion: string;
  serialNumber: string;
  hardwareId: string;
}

export class DeviceServiceClient {
  constructor(private soap: SoapClient) {}

  async getDeviceInformation(): Promise<DeviceInfo> {
    const response = await this.soap.request({
      service: 'device_service',
      action: 'GetDeviceInformation',
      body: '<tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>',
    });

    if (!response.success) {
      throw new Error(response.error?.reason || 'Failed to get device info');
    }

    const data = response.data as any;
    return {
      manufacturer: data.GetDeviceInformationResponse?.DeviceInformation?.Manufacturer || '',
      model: data.GetDeviceInformationResponse?.DeviceInformation?.Model || '',
      firmwareVersion: data.GetDeviceInformationResponse?.DeviceInformation?.FirmwareVersion || '',
      serialNumber: data.GetDeviceInformationResponse?.DeviceInformation?.SerialNumber || '',
      hardwareId: data.GetDeviceInformationResponse?.DeviceInformation?.HardwareId || '',
    };
  }

  async setHostname(name: string): Promise<void> {
    const body = `
      <tds:SetHostname xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
        <tds:Hostname>${escapeXml(name)}</tds:Hostname>
      </tds:SetHostname>
    `;

    const response = await this.soap.request({
      service: 'device_service',
      action: 'SetHostname',
      body,
    });

    if (!response.success) {
      throw new Error(response.error?.reason || 'Failed to set hostname');
    }
  }
}

function escapeXml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}
```

## React Hook for SOAP Requests

```typescript
// hooks/useSoapClient.ts

export function useSoapClient(baseUrl: string, auth?: AuthCredentials) {
  const soapClient = useRef<SoapClient | null>(null);

  useEffect(() => {
    soapClient.current = new SoapClient({
      baseUrl,
      username: auth?.username,
      password: auth?.password,
    });
  }, [baseUrl, auth]);

  return soapClient.current!;
}

// hooks/useDeviceInfo.ts

export function useDeviceInfo(camera: Camera) {
  const [data, setData] = useState<DeviceInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const soap = useSoapClient(camera.url, camera.auth);
  const deviceService = useMemo(() => new DeviceServiceClient(soap), [soap]);

  useEffect(() => {
    const fetch = async () => {
      try {
        setLoading(true);
        const info = await deviceService.getDeviceInformation();
        setData(info);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error');
        setData(null);
      } finally {
        setLoading(false);
      }
    };

    fetch();
  }, [deviceService]);

  return { data, loading, error };
}
```

## Error Handling

Handle SOAP faults gracefully:

```typescript
interface OnvifError {
  code: string;
  message: string;
  recoverable: boolean;
}

function mapSoapFaultToError(fault: SoapFault): OnvifError {
  const subcodeMap: Record<string, { message: string; recoverable: boolean }> = {
    'NotAuthorized': {
      message: 'Authentication failed. Check credentials.',
      recoverable: true,
    },
    'ActionNotSupported': {
      message: 'This operation is not supported by the camera.',
      recoverable: false,
    },
    'InvalidArgVal': {
      message: 'Invalid parameter value provided.',
      recoverable: true,
    },
    'ServiceUnavailable': {
      message: 'Camera service is temporarily unavailable.',
      recoverable: true,
    },
  };

  const mapped = subcodeMap[fault.subcode] || {
    message: fault.reason,
    recoverable: false,
  };

  return {
    code: fault.subcode,
    message: mapped.message,
    recoverable: mapped.recoverable,
  };
}
```

## Reference

For detailed SOAP patterns and service definitions, see `references/soap-patterns.md`.
