/**
 * SOAP Client Tests
 */

import { describe, it, expect } from 'vitest'
import { createSOAPEnvelope, parseSOAPResponse, soapBodies } from '@/services/soap/client'

describe('SOAP Client', () => {
  describe('createSOAPEnvelope', () => {
    it('should create a valid SOAP envelope with body content', () => {
      const body = '<tds:GetDeviceInformation />'
      const envelope = createSOAPEnvelope(body)

      expect(envelope).toContain('<?xml version="1.0" encoding="UTF-8"?>')
      expect(envelope).toContain('soap:Envelope')
      expect(envelope).toContain('soap:Body')
      expect(envelope).toContain(body)
    })

    it('should include all required ONVIF namespaces', () => {
      const envelope = createSOAPEnvelope('<test />')

      expect(envelope).toContain('xmlns:soap=')
      expect(envelope).toContain('xmlns:tds=')
      expect(envelope).toContain('xmlns:trt=')
      expect(envelope).toContain('xmlns:timg=')
      expect(envelope).toContain('xmlns:tptz=')
    })
  })

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
        </soap:Envelope>`

      const result = parseSOAPResponse<Record<string, unknown>>(xml)

      expect(result.success).toBe(true)
      expect(result.data).toBeDefined()
    })

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
        </soap:Envelope>`

      const result = parseSOAPResponse<Record<string, unknown>>(xml)

      expect(result.success).toBe(false)
      expect(result.fault).toBeDefined()
    })

    it('should return error for invalid XML', () => {
      const result = parseSOAPResponse<Record<string, unknown>>('not valid xml')

      expect(result.success).toBe(false)
      expect(result.fault?.code).toBe('ParseError')
    })

    it('should return error for missing envelope', () => {
      const xml = '<NoEnvelope />'
      const result = parseSOAPResponse<Record<string, unknown>>(xml)

      expect(result.success).toBe(false)
    })
  })

  describe('soapBodies', () => {
    it('should create GetDeviceInformation body', () => {
      const body = soapBodies.getDeviceInformation()
      expect(body).toContain('tds:GetDeviceInformation')
    })

    it('should create GetSystemDateAndTime body', () => {
      const body = soapBodies.getSystemDateAndTime()
      expect(body).toContain('tds:GetSystemDateAndTime')
    })

    it('should create GetNetworkInterfaces body', () => {
      const body = soapBodies.getNetworkInterfaces()
      expect(body).toContain('tds:GetNetworkInterfaces')
    })
  })
})
