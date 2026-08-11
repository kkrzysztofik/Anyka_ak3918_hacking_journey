/**
 * SOAP Client Tests
 */
import { describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import {
  createSOAPEnvelope,
  escapeXml,
  parseSOAPResponse,
  soapBodies,
  soapRequest,
} from '@/services/soap/client';

vi.mock('@/services/api', () => ({
  apiClient: {
    post: vi.fn(),
  },
}));

describe('SOAP Client', () => {
  describe('soapRequest', () => {
    it('should make a request and return partial response', async () => {
      vi.mocked(apiClient.post).mockResolvedValue({
        data: '<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><GetProfilesResponse><Profiles><Name>Profile1</Name></Profiles></GetProfilesResponse></soap:Body></soap:Envelope>',
        status: 200,
      });

      const result = await soapRequest<Record<string, unknown>>(
        '/test-endpoint',
        '<test />',
        'GetProfilesResponse',
      );

      expect(apiClient.post).toHaveBeenCalled();
      expect(result).toEqual({ Profiles: { Name: 'Profile1' } });
    });

    it('should throw error on failure', async () => {
      vi.mocked(apiClient.post).mockResolvedValue({
        data: '<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><soap:Fault><soap:Code><soap:Value>soap:Sender</soap:Value></soap:Code><soap:Reason><soap:Text>Error</soap:Text></soap:Reason></soap:Fault></soap:Body></soap:Envelope>',
        status: 200,
      });

      await expect(
        soapRequest('/test-endpoint', '<test />', 'GetProfilesResponse'),
      ).rejects.toThrow('Error');
    });
  });
  describe('createSOAPEnvelope', () => {
    it('should create a valid SOAP envelope with body content', () => {
      const body = '<tds:GetDeviceInformation />';
      const envelope = createSOAPEnvelope(body);

      expect(envelope).toContain('<?xml version="1.0" encoding="UTF-8"?>');
      expect(envelope).toContain('s:Envelope');
      expect(envelope).toContain('s:Body');
      expect(envelope).toContain(body);
    });

    it('should include all required ONVIF namespaces', () => {
      const envelope = createSOAPEnvelope('<test />');

      expect(envelope).toContain('xmlns:s=');
      expect(envelope).toContain('xmlns:tds=');
      expect(envelope).toContain('xmlns:trt=');
      expect(envelope).toContain('xmlns:timg=');
      expect(envelope).toContain('xmlns:tptz=');
      expect(envelope).toContain('xmlns:tt=');
    });

    it('should include tt namespace for ONVIF schema types', () => {
      const envelope = createSOAPEnvelope('<test />');
      expect(envelope).toContain('xmlns:tt="http://www.onvif.org/ver10/schema"');
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

  describe('soapBodies', () => {
    it('should create GetDeviceInformation body', () => {
      const body = soapBodies.getDeviceInformation();
      expect(body).toContain('tds:GetDeviceInformation');
    });

    it('should create GetSystemDateAndTime body', () => {
      const body = soapBodies.getSystemDateAndTime();
      expect(body).toContain('tds:GetSystemDateAndTime');
    });

    // PTZ SOAP bodies
    it('should create continuousMove body with velocity', () => {
      const body = soapBodies.continuousMove('ProfileToken1', 0.5, -0.3);
      expect(body).toContain('tptz:ContinuousMove');
      expect(body).toContain('ProfileToken1');
      expect(body).toContain('tt:PanTilt');
      expect(body).toContain('x="0.5"');
      expect(body).toContain('y="-0.3"');
    });

    it('should strip trailing zeros and a dangling decimal point from velocity', () => {
      // 1.0 rounds to "1.000" and 0.25 rounds to "0.250"; both must have
      // trailing zeros (and the dot, when fully whole) stripped.
      const body = soapBodies.continuousMove('ProfileToken1', 1, 0.25);
      expect(body).toContain('x="1"');
      expect(body).toContain('y="0.25"');
    });

    it('should format zero and non-finite velocity as "0"', () => {
      const body = soapBodies.continuousMove('ProfileToken1', 0, NaN);
      expect(body).toContain('x="0"');
      expect(body).toContain('y="0"');
    });

    it('should create ptzStop body', () => {
      const body = soapBodies.ptzStop('ProfileToken1');
      expect(body).toContain('tptz:Stop');
      expect(body).toContain('ProfileToken1');
      expect(body).toContain('<tptz:PanTilt>true</tptz:PanTilt>');
      expect(body).toContain('<tptz:Zoom>true</tptz:Zoom>');
    });

    it('should create gotoHomePosition body', () => {
      const body = soapBodies.gotoHomePosition('ProfileToken1');
      expect(body).toContain('tptz:GotoHomePosition');
      expect(body).toContain('ProfileToken1');
    });

    it('should create getPresets body', () => {
      const body = soapBodies.getPresets('ProfileToken1');
      expect(body).toContain('tptz:GetPresets');
      expect(body).toContain('ProfileToken1');
    });

    it('should create gotoPreset body', () => {
      const body = soapBodies.gotoPreset('ProfileToken1', 'preset1');
      expect(body).toContain('tptz:GotoPreset');
      expect(body).toContain('ProfileToken1');
      expect(body).toContain('preset1');
    });

    it('should create setPreset body with name only', () => {
      const body = soapBodies.setPreset('ProfileToken1', 'My Preset');
      expect(body).toContain('tptz:SetPreset');
      expect(body).toContain('ProfileToken1');
      expect(body).toContain('My Preset');
      expect(body).not.toContain('tptz:PresetToken');
    });

    it('should create setPreset body with existing preset token', () => {
      const body = soapBodies.setPreset('ProfileToken1', 'Updated', 'preset1');
      expect(body).toContain('tptz:SetPreset');
      expect(body).toContain('<tptz:PresetToken>preset1</tptz:PresetToken>');
      expect(body).toContain('Updated');
    });

    it('should create removePreset body', () => {
      const body = soapBodies.removePreset('ProfileToken1', 'preset1');
      expect(body).toContain('tptz:RemovePreset');
      expect(body).toContain('ProfileToken1');
      expect(body).toContain('preset1');
    });

    it('should escape XML in PTZ soap bodies', () => {
      const body = soapBodies.setPreset('Profile<Token>', 'Name&"Test"');
      expect(body).toContain('Profile&lt;Token&gt;');
      expect(body).toContain('Name&amp;&quot;Test&quot;');
    });
  });
});
