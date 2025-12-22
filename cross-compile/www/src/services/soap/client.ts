/**
 * SOAP Client for ONVIF communication
 *
 * Provides utilities for building SOAP envelopes and parsing XML responses.
 * Uses fast-xml-parser for XML handling.
 */
import { XMLBuilder, XMLParser } from 'fast-xml-parser';

// SOAP namespace declarations
// These are XML namespace URIs (identifiers), not network protocols
// NOSONAR: S5332 - These are XML namespace identifiers, not actual HTTP connections
const SOAP_NS = 'http://www.w3.org/2003/05/soap-envelope'; // NOSONAR
const DEVICE_NS = 'http://www.onvif.org/ver10/device/wsdl'; // NOSONAR
const MEDIA_NS = 'http://www.onvif.org/ver10/media/wsdl'; // NOSONAR
const IMAGING_NS = 'http://www.onvif.org/ver20/imaging/wsdl'; // NOSONAR
const PTZ_NS = 'http://www.onvif.org/ver20/ptz/wsdl'; // NOSONAR

export interface SOAPFault {
  code: string;
  subcode?: string;
  reason: string;
}

export interface SOAPResponse<T> {
  success: boolean;
  data?: T;
  fault?: SOAPFault;
}

// XML Parser configuration
const parserOptions = {
  ignoreAttributes: false,
  attributeNamePrefix: '@_',
  removeNSPrefix: true,
  parseTagValue: true,
  trimValues: true,
};

const parser = new XMLParser(parserOptions);

const builderOptions = {
  ignoreAttributes: false,
  attributeNamePrefix: '@_',
  format: false,
  suppressEmptyNode: true,
};

const builder = new XMLBuilder(builderOptions);

/**
 * Creates a SOAP 1.2 envelope with the given body content
 */
export function createSOAPEnvelope(body: string): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="${SOAP_NS}" xmlns:tds="${DEVICE_NS}" xmlns:trt="${MEDIA_NS}" xmlns:timg="${IMAGING_NS}" xmlns:tptz="${PTZ_NS}">
  <soap:Body>
    ${body}
  </soap:Body>
</soap:Envelope>`;
}

/**
 * Parses a SOAP XML response and extracts the body content
 */
export function parseSOAPResponse<T>(xml: string): SOAPResponse<T> {
  try {
    const parsed = parser.parse(xml);

    // Navigate to SOAP Body
    const envelope = parsed.Envelope || parsed['soap:Envelope'];
    if (!envelope) {
      return {
        success: false,
        fault: { code: 'ParseError', reason: 'Invalid SOAP envelope' },
      };
    }

    const body = envelope.Body || envelope['soap:Body'];
    if (!body) {
      return {
        success: false,
        fault: { code: 'ParseError', reason: 'Missing SOAP body' },
      };
    }

    // Check for SOAP Fault
    const fault = extractSOAPFault(body);
    if (fault) {
      return { success: false, fault };
    }

    // Return the body content (excluding Fault)
    return { success: true, data: body as T };
  } catch (error) {
    return {
      success: false,
      fault: {
        code: 'ParseError',
        reason: error instanceof Error ? error.message : 'XML parsing failed',
      },
    };
  }
}

/**
 * Extracts SOAP Fault information from response body
 */
function extractSOAPFault(body: Record<string, unknown>): SOAPFault | null {
  const fault = body.Fault || body['soap:Fault'];
  if (!fault || typeof fault !== 'object') {
    return null;
  }

  const faultObj = fault as Record<string, unknown>;

  // Extract fault code
  const code = faultObj.Code || faultObj['soap:Code'];
  const codeObj = code as Record<string, unknown> | undefined;
  const codeValue = codeObj?.Value || codeObj?.['soap:Value'] || 'Unknown';

  // Extract subcode if present
  const subcode = codeObj?.Subcode || codeObj?.['soap:Subcode'];
  const subcodeObj = subcode as Record<string, unknown> | undefined;
  const subcodeValue = subcodeObj?.Value || subcodeObj?.['soap:Value'];

  // Extract reason
  const reason = faultObj.Reason || faultObj['soap:Reason'];
  const reasonObj = reason as Record<string, unknown> | undefined;
  const text = reasonObj?.Text || reasonObj?.['soap:Text'] || 'Unknown error';
  const reasonText =
    typeof text === 'object' ? (text as Record<string, unknown>)['#text'] || text : text;

  return {
    code: String(codeValue),
    subcode: subcodeValue ? String(subcodeValue) : undefined,
    reason: String(reasonText),
  };
}

/**
 * SOAP request body builders for common ONVIF operations
 */
export const soapBodies = {
  getDeviceInformation: () => '<tds:GetDeviceInformation />',
  getSystemDateAndTime: () => '<tds:GetSystemDateAndTime />',
  getNetworkInterfaces: () => '<tds:GetNetworkInterfaces />',
  getDNS: () => '<tds:GetDNS />',
  getUsers: () => '<tds:GetUsers />',
  getProfiles: () => '<trt:GetProfiles />',
  getImagingSettings: (videoSourceToken: string) =>
    `<timg:GetImagingSettings><timg:VideoSourceToken>${videoSourceToken}</timg:VideoSourceToken></timg:GetImagingSettings>`,
  systemReboot: () => '<tds:SystemReboot />',
  setSystemFactoryDefault: (type: 'Hard' | 'Soft') =>
    `<tds:SetSystemFactoryDefault><tds:FactoryDefault>${type}</tds:FactoryDefault></tds:SetSystemFactoryDefault>`,
  getSystemBackup: () => '<tds:GetSystemBackup />',
  restoreSystem: (backupFiles: Array<{ Name: string; Data: string }>) => {
    const filesXml = backupFiles
      .map(
        (file) =>
          `<tds:BackupFile><tds:Name>${file.Name}</tds:Name><tds:Data>${file.Data}</tds:Data></tds:BackupFile>`,
      )
      .join('');
    return `<tds:RestoreSystem><tds:BackupFiles>${filesXml}</tds:BackupFiles></tds:RestoreSystem>`;
  },
};

export { parser, builder };
