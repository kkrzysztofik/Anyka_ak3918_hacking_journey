---
name: onvif-soap-client
description: Use when building or extending TypeScript SOAP clients for the camera WebUI that talk to ONVIF services (soapRequest, createSOAPEnvelope, parseSOAPResponse, soapBodies, XML parsing, Basic Auth, service clients).
version: 2.0.0
---

# ONVIF SOAP Client (WebUI)

Build/extend SOAP clients in `cross-compile/www`. The real client is **functional** (`soapRequest`) using **fast-xml-parser** and **HTTP Basic Auth** — there is no WS-Security `SoapClient` class. Verify against `www/src/services/soap/client.ts`.

## Core API

`www/src/services/soap/client.ts`:

```typescript
// Performs a SOAP request to an ONVIF endpoint. Returns extracted data or whole body.
export async function soapRequest<T>(
  endpoint: string,        // e.g. ENDPOINTS.device
  body: string,            // XML body content (inside <s:Body>)
  responseTarget?: string, // e.g. 'GetProfilesResponse' — key to extract from response
): Promise<T>

export function createSOAPEnvelope(body: string): string;      // SOAP 1.2 envelope
export function parseSOAPResponse<T>(xml: string): SOAPResponse<T>;  // {success, data?, fault?}
export interface SOAPFault { code: string; subcode?: string; reason: string; }
export function escapeXml(input: string): string;              // & < > " '
export function escapeXmlAttribute(input: string): string;     // alias of escapeXml
```

`soapRequest` throws `Error` on failure (fault reason or "response target not found"). It does **not** return `{success: false}` on error.

## SOAP Envelope / Response Shape

```xml
<!-- Request body (SOAP 1.2) -->
<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tds="http://www.onvif.org/ver10/device/wsdl"
            xmlns:trt="http://www.onvif.org/ver10/media/wsdl"
            xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl"
            xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"
            xmlns:tt="http://www.onvif.org/ver10/schema">
  <s:Body>...</s:Body>
</s:Envelope>
```

`parseSOAPResponse` uses an `XMLParser` with `attributeNamePrefix: '@_'`, `removeNSPrefix: true`, `parseTagValue: true`. Attributes arrive as `@_attr`, repeated elements become arrays. It navigates `Envelope → Body`, detects `Fault`, and returns the body content.

## Body Builders (`soapBodies`)

`client.ts` exports `soapBodies`, pre-built body fragments:

```typescript
soapBodies.getDeviceInformation();               // '<tds:GetDeviceInformation />'
soapBodies.getProfiles();                        // '<trt:GetProfiles />'
soapBodies.getImagingSettings(videoSourceToken);
soapBodies.systemReboot();
soapBodies.setSystemFactoryDefault('Hard' | 'Soft');
soapBodies.continuousMove(profileToken, panSpeed, tiltSpeed);
soapBodies.removePreset(profileToken, presetToken);
soapBodies.sendAuxiliaryCommand(profileToken, auxiliaryData);
```

Always escape user-supplied values with `escapeXml` when interpolating into bodies.

## Endpoints & Transport

`www/src/services/api.ts`:

```typescript
export const ENDPOINTS = {
  device: '/onvif/device_service',
  media: '/onvif/media_service',
  imaging: '/onvif/imaging_service',
  ptz: '/onvif/ptz_service',
} as const;
```

`apiClient.post(url, body, config?)` is a fetch wrapper that:
- sets `Content-Type: application/soap+xml; charset=utf-8` and `Accept: application/soap+xml, application/xml, */*`
- injects an `Authorization` header from a registered getter (Basic Auth) unless one is already set
- times out after `DEFAULT_TIMEOUT_MS` (10s), honors `AbortSignal`
- on HTTP 401 clears `onvif_camera_auth` from sessionStorage and dispatches `auth:unauthorized`; throws `ApiError(status, data)` for non-OK responses

## Authentication — Basic Auth (no WS-Security)

Credential verification lives in `www/src/services/authService.ts`:

```typescript
import { apiClient, ENDPOINTS } from '@/services/api';
import { createSOAPEnvelope, soapBodies, parseSOAPResponse } from '@/services/soap/client';

const credentials = `${username}:${password}`;
const authHeader = `Basic ${btoa(credentials)}`;
const response = await apiClient.post(ENDPOINTS.device, createSOAPEnvelope(soapBodies.getDeviceInformation()), {
  headers: { Authorization: authHeader },
});
const parsed = parseSOAPResponse(response.data);
```

For app-wide requests, `App.tsx` wires the auth getter: `setAuthHeaderGetter(getBasicAuthHeader)` from `useAuth`. New requests must not build their own header when the global getter is registered.

## Service Clients

Per-service clients are thin wrappers around `soapRequest`, e.g. `www/src/services/deviceService.ts`, `ptzService.ts`, `imagingService.ts`:

```typescript
export async function getDeviceInformation(): Promise<DeviceInfo> {
  return soapRequest<...>(ENDPOINTS.device, soapBodies.getDeviceInformation(), 'GetDeviceInformationResponse');
}
```

See `www/src/services/api.test.ts` for the fetch-stub pattern and `authService.test.ts` for auth tests.

## XML Parsing Notes

- `processEntities: false` — parser does not expand XML entities; escape on output instead.
- `suppressEmptyNode: true` on the builder.
- For `parseSOAPResponse`, prefer it over hand-rolled DOM parsing; it already handles SOAP 1.2 namespace variants (`Envelope` / `soap:Envelope`).

## Testing

Follow `www/src/services/soap/client.test.ts`: `vi.mock('@/services/api')` then stub `apiClient.post` to resolve `{ data: '<soap envelope>', status: 200 }`. Quality gates: `cd cross-compile/www && npm run lint && npm run type-check && npm run test`.
