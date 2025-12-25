/**
 * SOAP Client Tests
 */
import { describe, expect, it } from 'vitest';

import {
  createSOAPEnvelope,
  escapeXml,
  escapeXmlAttribute,
  parseSOAPResponse,
  soapBodies,
} from '@/services/soap/client';

describe('SOAP Client', () => {
  describe('createSOAPEnvelope', () => {
    it('should create a valid SOAP envelope with body content', () => {
      const body = '<tds:GetDeviceInformation />';
      const envelope = createSOAPEnvelope(body);

      expect(envelope).toContain('<?xml version="1.0" encoding="UTF-8"?>');
      expect(envelope).toContain('soap:Envelope');
      expect(envelope).toContain('soap:Body');
      expect(envelope).toContain(body);
    });

    it('should include all required ONVIF namespaces', () => {
      const envelope = createSOAPEnvelope('<test />');

      expect(envelope).toContain('xmlns:soap=');
      expect(envelope).toContain('xmlns:tds=');
      expect(envelope).toContain('xmlns:trt=');
      expect(envelope).toContain('xmlns:timg=');
      expect(envelope).toContain('xmlns:tptz=');
    });
  });

  describe('parseSOAPResponse', () => {
    it('should parse a successful SOAP response', () => {
      const xml = `<?xml version="1.0" encoding="UTF-8"?>
        <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
          <soap:Body>
            <GetDeviceInformationResponse>
              <Manufacturer>Anyka</Manufacturer>
              <Model>AK3918E</Model>
            </GetDeviceInformationResponse>
          </soap:Body>
        </soap:Envelope>`;

      const result = parseSOAPResponse<Record<string, unknown>>(xml);

      expect(result.success).toBe(true);
      expect(result.data).toBeDefined();
    });

    it('should handle SOAP faults', () => {
      const xml = `<?xml version="1.0" encoding="UTF-8"?>
        <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
          <soap:Body>
            <soap:Fault>
              <soap:Code>
                <soap:Value>soap:Sender</soap:Value>
              </soap:Code>
              <soap:Reason>
                <soap:Text>Invalid operation</soap:Text>
              </soap:Reason>
            </soap:Fault>
          </soap:Body>
        </soap:Envelope>`;

      const result = parseSOAPResponse<Record<string, unknown>>(xml);

      expect(result.success).toBe(false);
      expect(result.fault).toBeDefined();
    });

    it('should return error for invalid XML', () => {
      const result = parseSOAPResponse<Record<string, unknown>>('not valid xml');

      expect(result.success).toBe(false);
      expect(result.fault?.code).toBe('ParseError');
    });

    it('should return error for missing envelope', () => {
      const xml = '<NoEnvelope />';
      const result = parseSOAPResponse<Record<string, unknown>>(xml);

      expect(result.success).toBe(false);
      expect(result.fault?.code).toBe('ParseError');
    });

    it('should return error for missing SOAP body', () => {
      const xml = `<?xml version="1.0" encoding="UTF-8"?>
        <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
        </soap:Envelope>`;

      const result = parseSOAPResponse<Record<string, unknown>>(xml);

      expect(result.success).toBe(false);
      expect(result.fault?.code).toBe('ParseError');
      // The parser might return empty body object, so check for either missing body or empty body
      expect(result.fault?.reason).toMatch(/Missing SOAP body|Invalid SOAP envelope/);
    });

    it('should handle SOAP fault with subcode', () => {
      const xml = `<?xml version="1.0" encoding="UTF-8"?>
        <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
          <soap:Body>
            <soap:Fault>
              <soap:Code>
                <soap:Value>soap:Sender</soap:Value>
                <soap:Subcode>
                  <soap:Value>InvalidParameter</soap:Value>
                </soap:Subcode>
              </soap:Code>
              <soap:Reason>
                <soap:Text>Invalid parameter value</soap:Text>
              </soap:Reason>
            </soap:Fault>
          </soap:Body>
        </soap:Envelope>`;

      const result = parseSOAPResponse<Record<string, unknown>>(xml);

      expect(result.success).toBe(false);
      expect(result.fault).toBeDefined();
      expect(result.fault?.code).toBe('soap:Sender');
      expect(result.fault?.subcode).toBe('InvalidParameter');
      expect(result.fault?.reason).toBe('Invalid parameter value');
    });

    it('should handle SOAP fault without namespace prefix', () => {
      const xml = `<?xml version="1.0" encoding="UTF-8"?>
        <Envelope xmlns="http://www.w3.org/2003/05/soap-envelope">
          <Body>
            <Fault>
              <Code>
                <Value>soap:Sender</Value>
              </Code>
              <Reason>
                <Text>Error message</Text>
              </Reason>
            </Fault>
          </Body>
        </Envelope>`;

      const result = parseSOAPResponse<Record<string, unknown>>(xml);

      expect(result.success).toBe(false);
      expect(result.fault).toBeDefined();
      expect(result.fault?.code).toBe('soap:Sender');
    });

    it('should handle parsing error with error message', () => {
      // Create invalid XML that will cause parsing error
      const invalidXml = '<?xml version="1.0"?><unclosed>';

      const result = parseSOAPResponse<Record<string, unknown>>(invalidXml);

      expect(result.success).toBe(false);
      expect(result.fault?.code).toBe('ParseError');
      expect(result.fault?.reason).toBeDefined();
    });
  });

  describe('escapeXml', () => {
    it('should escape all XML special characters', () => {
      expect(escapeXml('test & value < > " \'')).toBe('test &amp; value &lt; &gt; &quot; &apos;');
    });

    it('should handle empty string', () => {
      expect(escapeXml('')).toBe('');
    });

    it('should handle string without special characters', () => {
      expect(escapeXml('normal text')).toBe('normal text');
    });
  });

  describe('escapeXmlAttribute', () => {
    it('should escape XML attribute values', () => {
      expect(escapeXmlAttribute('test & value < > " \'')).toBe(
        'test &amp; value &lt; &gt; &quot; &apos;',
      );
    });
  });

  describe('soapBodies', () => {
    it('should create GetDeviceInformation body', () => {
      const body = soapBodies.getDeviceInformation();
      expect(body).toContain('tds:GetDeviceInformation');
    });

    it('should create GetSystemDateAndTime body', () => {
      const body = soapBodies.getSystemDateAndTime();
      expect(body).toContain('tds:GetSystemDateAndTime');
    });

    it('should create GetNetworkInterfaces body', () => {
      const body = soapBodies.getNetworkInterfaces();
      expect(body).toContain('tds:GetNetworkInterfaces');
    });

    it('should create GetDNS body', () => {
      const body = soapBodies.getDNS();
      expect(body).toContain('tds:GetDNS');
    });

    it('should create GetUsers body', () => {
      const body = soapBodies.getUsers();
      expect(body).toContain('tds:GetUsers');
    });

    it('should create GetProfiles body', () => {
      const body = soapBodies.getProfiles();
      expect(body).toContain('trt:GetProfiles');
    });

    it('should create GetImagingSettings body with token', () => {
      const body = soapBodies.getImagingSettings('VideoSourceToken1');
      expect(body).toContain('timg:GetImagingSettings');
      expect(body).toContain('VideoSourceToken1');
    });

    it('should create systemReboot body', () => {
      const body = soapBodies.systemReboot();
      expect(body).toContain('tds:SystemReboot');
    });

    it('should create setSystemFactoryDefault body with Hard type', () => {
      const body = soapBodies.setSystemFactoryDefault('Hard');
      expect(body).toContain('tds:SetSystemFactoryDefault');
      expect(body).toContain('Hard');
    });

    it('should create setSystemFactoryDefault body with Soft type', () => {
      const body = soapBodies.setSystemFactoryDefault('Soft');
      expect(body).toContain('tds:SetSystemFactoryDefault');
      expect(body).toContain('Soft');
    });

    it('should create getSystemBackup body', () => {
      const body = soapBodies.getSystemBackup();
      expect(body).toContain('tds:GetSystemBackup');
    });

    it('should create restoreSystem body with single backup file', () => {
      const backupFiles = [{ Name: 'config.toml', Data: 'base64data' }];
      const body = soapBodies.restoreSystem(backupFiles);
      expect(body).toContain('tds:RestoreSystem');
      expect(body).toContain('config.toml');
      expect(body).toContain('base64data');
    });

    it('should create restoreSystem body with multiple backup files', () => {
      const backupFiles = [
        { Name: 'config.toml', Data: 'data1' },
        { Name: 'settings.json', Data: 'data2' },
      ];
      const body = soapBodies.restoreSystem(backupFiles);
      expect(body).toContain('tds:RestoreSystem');
      expect(body).toContain('config.toml');
      expect(body).toContain('settings.json');
      expect(body).toContain('data1');
      expect(body).toContain('data2');
    });

    it('should create restoreSystem body with empty backup files array', () => {
      const backupFiles: Array<{ Name: string; Data: string }> = [];
      const body = soapBodies.restoreSystem(backupFiles);
      expect(body).toContain('tds:RestoreSystem');
      expect(body).toContain('tds:BackupFiles');
    });
  });
});
