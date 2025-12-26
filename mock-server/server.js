/**
 * ONVIF Mock Server for Local Development
 *
 * Simple Node.js server that returns static SOAP responses.
 *
 * Usage: node server.js
 * Runs on port 8081 by default.
 */

import http from 'http'

const PORT = process.env.PORT || 8081

// Static SOAP responses for common operations
const responses = {
  GetDeviceInformation: `<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <soap:Body>
    <tds:GetDeviceInformationResponse>
      <tds:Manufacturer>Anyka</tds:Manufacturer>
      <tds:Model>AK3918</tds:Model>
      <tds:FirmwareVersion>1.0.0-dev</tds:FirmwareVersion>
      <tds:SerialNumber>DEV-MOCK-001</tds:SerialNumber>
      <tds:HardwareId>ak3918-mock</tds:HardwareId>
    </tds:GetDeviceInformationResponse>
  </soap:Body>
</soap:Envelope>`,

  GetSystemDateAndTime: `<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <soap:Body>
    <tds:GetSystemDateAndTimeResponse>
      <tds:SystemDateAndTime>
        <tt:DateTimeType>NTP</tt:DateTimeType>
        <tt:DaylightSavings>false</tt:DaylightSavings>
        <tt:TimeZone>
          <tt:TZ>UTC+0</tt:TZ>
        </tt:TimeZone>
        <tt:UTCDateTime>
          <tt:Time>
            <tt:Hour>12</tt:Hour>
            <tt:Minute>0</tt:Minute>
            <tt:Second>0</tt:Second>
          </tt:Time>
          <tt:Date>
            <tt:Year>2025</tt:Year>
            <tt:Month>12</tt:Month>
            <tt:Day>16</tt:Day>
          </tt:Date>
        </tt:UTCDateTime>
      </tds:SystemDateAndTime>
    </tds:GetSystemDateAndTimeResponse>
  </soap:Body>
</soap:Envelope>`,

  GetUsers: `<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <soap:Body>
    <tds:GetUsersResponse>
      <tds:User>
        <tt:Username>admin</tt:Username>
        <tt:UserLevel>Administrator</tt:UserLevel>
      </tds:User>
    </tds:GetUsersResponse>
  </soap:Body>
</soap:Envelope>`,

  GetProfiles: `<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <soap:Body>
    <trt:GetProfilesResponse>
      <trt:Profiles token="profile_1" fixed="true">
        <tt:Name>MainStream</tt:Name>
      </trt:Profiles>
      <trt:Profiles token="profile_2" fixed="true">
        <tt:Name>SubStream</tt:Name>
      </trt:Profiles>
    </trt:GetProfilesResponse>
  </soap:Body>
</soap:Envelope>`,
}

// Detect operation from SOAP body
function detectOperation(body) {
  for (const op of Object.keys(responses)) {
    if (body.includes(op)) {
      return op
    }
  }
  return null
}

const server = http.createServer((req, res) => {
  // CORS headers for development
  res.setHeader('Access-Control-Allow-Origin', '*')
  res.setHeader('Access-Control-Allow-Methods', 'POST, OPTIONS')
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization')

  if (req.method === 'OPTIONS') {
    res.writeHead(204)
    res.end()
    return
  }

  if (req.method !== 'POST') {
    res.writeHead(405, { 'Content-Type': 'text/plain' })
    res.end('Method Not Allowed')
    return
  }

  let body = ''
  req.on('data', chunk => {
    body += chunk.toString()
  })

  req.on('end', () => {
    const operation = detectOperation(body)

    if (operation && responses[operation]) {
      console.log(`[MOCK] ${req.url} -> ${operation}`)
      res.writeHead(200, { 'Content-Type': 'application/soap+xml; charset=utf-8' })
      res.end(responses[operation])
    } else {
      console.log(`[MOCK] ${req.url} -> Unknown operation`)
      res.writeHead(400, { 'Content-Type': 'text/plain' })
      res.end('Unknown ONVIF operation')
    }
  })
})

server.listen(PORT, () => {
  console.log(`ONVIF Mock Server running on http://localhost:${PORT}`)
  console.log('Endpoints:')
  console.log('  POST /onvif/device_service')
  console.log('  POST /onvif/media_service')
})
