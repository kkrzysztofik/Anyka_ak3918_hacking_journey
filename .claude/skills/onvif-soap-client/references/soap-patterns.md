# SOAP Client Patterns Reference

## Common ONVIF Service Requests

### Device Service - GetCapabilities

```xml
<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
    <s:Body>
        <tds:GetCapabilities>
            <tds:Category>All</tds:Category>
        </tds:GetCapabilities>
    </s:Body>
</s:Envelope>
```

### Media Service - GetProfiles

```xml
<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
    <s:Body>
        <trt:GetProfiles/>
    </s:Body>
</s:Envelope>
```

### PTZ Service - GotoHomePosition

```xml
<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
    <s:Body>
        <tptz:GotoHomePosition>
            <tptz:ProfileToken>profile1</tptz:ProfileToken>
            <tptz:Speed>
                <tt:PanTilt x="0.5" y="0.5" xmlns:tt="http://www.onvif.org/ver10/schema"/>
                <tt:Zoom x="0.5" xmlns:tt="http://www.onvif.org/ver10/schema"/>
            </tptz:Speed>
        </tptz:GotoHomePosition>
    </s:Body>
</s:Envelope>
```

## TypeScript SOAP Builders

### Build request dynamically

```typescript
export class SoapBuilder {
  static createDeviceServiceRequest(action: string, params: Record<string, any>): string {
    const body = this.buildBody(action, params, 'http://www.onvif.org/ver10/device/wsdl', 'tds');
    return this.wrapEnvelope(body);
  }

  static createMediaServiceRequest(action: string, params: Record<string, any>): string {
    const body = this.buildBody(action, params, 'http://www.onvif.org/ver10/media/wsdl', 'trt');
    return this.wrapEnvelope(body);
  }

  private static buildBody(
    action: string,
    params: Record<string, any>,
    namespace: string,
    prefix: string
  ): string {
    let xml = `<${prefix}:${action} xmlns:${prefix}="${namespace}">`;

    for (const [key, value] of Object.entries(params)) {
      xml += this.buildElement(key, value, prefix);
    }

    xml += `</${prefix}:${action}>`;
    return xml;
  }

  private static buildElement(key: string, value: any, prefix: string): string {
    if (value === null || value === undefined) {
      return '';
    }

    if (typeof value === 'object') {
      let xml = `<${prefix}:${key}>`;
      for (const [k, v] of Object.entries(value)) {
        xml += this.buildElement(k, v, prefix);
      }
      xml += `</${prefix}:${key}>`;
      return xml;
    }

    return `<${prefix}:${key}>${escapeXml(String(value))}</${prefix}:${key}>`;
  }

  private static wrapEnvelope(body: string): string {
    return `<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body>
        ${body}
    </s:Body>
</s:Envelope>`;
  }
}
```

## Response Parsing

### Extract specific field from response

```typescript
export function extractFromResponse<T>(
  xml: string,
  path: string[]
): T | null {
  try {
    const parser = new DOMParser();
    const doc = parser.parseFromString(xml, 'text/xml');

    let current: Element | null = doc.documentElement;

    for (const segment of path) {
      if (!current) return null;

      // Try with namespace and without
      const childName = segment.includes(':') ? segment : `*[local-name()='${segment}']`;
      current = current.querySelector(childName);
    }

    return (current?.textContent as T) || null;
  } catch {
    return null;
  }
}

// Usage
const manufacturer = extractFromResponse<string>(
  responseXml,
  ['GetDeviceInformationResponse', 'DeviceInformation', 'Manufacturer']
);
```

### Extract array of elements

```typescript
export function extractArray<T>(
  xml: string,
  rootPath: string[],
  itemPath: string,
  mapper: (el: Element) => T
): T[] {
  try {
    const parser = new DOMParser();
    const doc = parser.parseFromString(xml, 'text/xml');

    let current: Element | null = doc.documentElement;

    for (const segment of rootPath) {
      if (!current) return [];
      current = current.querySelector(`*[local-name()='${segment}']`);
    }

    if (!current) return [];

    const items = current.querySelectorAll(`*[local-name()='${itemPath}']`);
    return Array.from(items).map(mapper);
  } catch {
    return [];
  }
}

// Usage
const profiles = extractArray<MediaProfile>(
  responseXml,
  ['GetProfilesResponse'],
  'Profiles',
  (el) => ({
    token: el.getAttribute('token') || '',
    name: el.querySelector('*[local-name()="Name"]')?.textContent || '',
  })
);
```

## HTTP Digest Authentication

Alternative to WS-Security:

```typescript
export class HttpDigestAuth {
  private nonce: string = '';
  private nc: number = 0;
  private cnonce: string = '';

  async withDigestAuth(
    url: string,
    method: string,
    body: string,
    username: string,
    password: string
  ): Promise<Response> {
    // First request to get realm/nonce
    let response = await fetch(url, {
      method,
      headers: {
        'Content-Type': 'text/xml; charset=utf-8',
      },
      body,
    });

    // If 401, extract challenge and retry with auth
    if (response.status === 401) {
      const wwwAuth = response.headers.get('www-authenticate') || '';
      const challenge = this.parseChallenge(wwwAuth);

      const authHeader = this.buildAuthHeader(
        username,
        password,
        method,
        url,
        body,
        challenge
      );

      response = await fetch(url, {
        method,
        headers: {
          'Content-Type': 'text/xml; charset=utf-8',
          Authorization: authHeader,
        },
        body,
      });
    }

    return response;
  }

  private parseChallenge(wwwAuth: string): Record<string, string> {
    const parts = wwwAuth.split(' ').slice(1);
    const challenge: Record<string, string> = {};

    for (const part of parts) {
      const [key, value] = part.split('=');
      challenge[key.trim()] = value.replace(/"/g, '');
    }

    return challenge;
  }

  private buildAuthHeader(
    username: string,
    password: string,
    method: string,
    url: string,
    body: string,
    challenge: Record<string, string>
  ): string {
    const realm = challenge.realm || '';
    const nonce = challenge.nonce || '';
    const qop = challenge.qop || '';

    this.nc++;
    this.cnonce = this.generateNonce();

    const ha1 = this.md5(`${username}:${realm}:${password}`);
    const ha2 = this.md5(`${method}:${url}`);
    const response = qop
      ? this.md5(`${ha1}:${nonce}:${this.nc}:${this.cnonce}:auth:${ha2}`)
      : this.md5(`${ha1}:${nonce}:${ha2}`);

    let authHeader = `Digest username="${username}", realm="${realm}", nonce="${nonce}", uri="${url}", response="${response}"`;

    if (qop) {
      authHeader += `, qop=auth, nc=${this.nc}, cnonce="${this.cnonce}"`;
    }

    return authHeader;
  }

  private md5(str: string): string {
    // In browser, would use SubtleCrypto API
    // For Node.js: const crypto = require('crypto');
    // return crypto.createHash('md5').update(str).digest('hex');
    return '';
  }

  private generateNonce(): string {
    return Math.random().toString(36).substring(2);
  }
}
```

## Retry and Timeout Logic

```typescript
export class ResilientSoapClient extends SoapClient {
  async request<T>(
    req: SoapRequest,
    options: RequestOptions = {}
  ): Promise<SoapResponse<T>> {
    const {
      maxRetries = 3,
      timeout = 10000,
      backoffMs = 1000,
    } = options;

    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        return await this.requestWithTimeout<T>(req, timeout);
      } catch (error) {
        lastError = error as Error;

        if (attempt < maxRetries && this.isRetryable(error)) {
          const delay = backoffMs * Math.pow(2, attempt);
          console.warn(
            `Attempt ${attempt + 1} failed, retrying in ${delay}ms...`,
            error
          );
          await new Promise((r) => setTimeout(r, delay));
        } else {
          break;
        }
      }
    }

    return {
      success: false,
      error: {
        code: 'RequestFailed',
        subcode: 'Exhausted',
        reason: lastError?.message || 'All retries exhausted',
      },
    };
  }

  private async requestWithTimeout<T>(
    req: SoapRequest,
    timeout: number
  ): Promise<SoapResponse<T>> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeout);

    try {
      return await super.request<T>(req);
    } finally {
      clearTimeout(timeoutId);
    }
  }

  private isRetryable(error: unknown): boolean {
    if (error instanceof TypeError) {
      return error.message.includes('fetch') || error.message.includes('timeout');
    }
    return false;
  }
}
```

## Service Registry

```typescript
export class ServiceRegistry {
  private services: Map<string, ServiceEndpoint> = new Map();

  registerService(name: string, endpoint: ServiceEndpoint): void {
    this.services.set(name, endpoint);
  }

  getService(name: string): ServiceEndpoint | undefined {
    return this.services.get(name);
  }

  getAllServices(): ServiceEndpoint[] {
    return Array.from(this.services.values());
  }
}

export interface ServiceEndpoint {
  service: string;
  namespace: string;
  prefix: string;
  operations: string[];
}

// Initialize
const registry = new ServiceRegistry();

registry.registerService('device', {
  service: 'device_service',
  namespace: 'http://www.onvif.org/ver10/device/wsdl',
  prefix: 'tds',
  operations: ['GetDeviceInformation', 'SetHostname', 'GetCapabilities'],
});

registry.registerService('media', {
  service: 'media_service',
  namespace: 'http://www.onvif.org/ver10/media/wsdl',
  prefix: 'trt',
  operations: ['GetProfiles', 'CreateProfile', 'GetStreamUri'],
});
```
