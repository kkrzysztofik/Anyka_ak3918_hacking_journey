/**
 * Time Service
 *
 * SOAP operations for system date/time configuration.
 */

import { apiClient, ENDPOINTS } from '@/services/api'
import { createSOAPEnvelope, parseSOAPResponse, soapBodies } from '@/services/soap/client'

export type DateTimeType = 'NTP' | 'Manual'

export interface SystemDateTime {
  dateTimeType: DateTimeType
  daylightSavings: boolean
  timezone: string
  utcDateTime: Date
}

/**
 * Get system date and time configuration
 */
export async function getSystemDateAndTime(): Promise<SystemDateTime> {
  const envelope = createSOAPEnvelope(soapBodies.getSystemDateAndTime())

  const response = await apiClient.post(ENDPOINTS.device, envelope)
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data)

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get system date/time')
  }

  const data = parsed.data?.GetSystemDateAndTimeResponse as Record<string, unknown> | undefined
  const sdt = data?.SystemDateAndTime as Record<string, unknown> | undefined

  if (!sdt) {
    throw new Error('Invalid response: missing SystemDateAndTime')
  }

  const utcDateTime = sdt.UTCDateTime as Record<string, unknown> | undefined
  const time = utcDateTime?.Time as Record<string, unknown> | undefined
  const date = utcDateTime?.Date as Record<string, unknown> | undefined
  const timezone = sdt.TimeZone as Record<string, unknown> | undefined

  // Build Date object from response
  const year = Number(date?.Year || new Date().getFullYear())
  const month = Number(date?.Month || 1) - 1 // JS months are 0-indexed
  const day = Number(date?.Day || 1)
  const hour = Number(time?.Hour || 0)
  const minute = Number(time?.Minute || 0)
  const second = Number(time?.Second || 0)

  return {
    dateTimeType: String(sdt.DateTimeType || 'NTP') as DateTimeType,
    daylightSavings: sdt.DaylightSavings === true || sdt.DaylightSavings === 'true',
    timezone: String(timezone?.TZ || 'UTC'),
    utcDateTime: new Date(Date.UTC(year, month, day, hour, minute, second)),
  }
}

/**
 * Set system date and time configuration
 */
export async function setSystemDateAndTime(
  dateTimeType: DateTimeType,
  daylightSavings: boolean,
  timezone: string,
  manualDateTime?: Date
): Promise<void> {
  let utcDateTimeXml = ''

  if (dateTimeType === 'Manual' && manualDateTime) {
    const d = manualDateTime
    utcDateTimeXml = `
      <tds:UTCDateTime>
        <tt:Time>
          <tt:Hour>${d.getUTCHours()}</tt:Hour>
          <tt:Minute>${d.getUTCMinutes()}</tt:Minute>
          <tt:Second>${d.getUTCSeconds()}</tt:Second>
        </tt:Time>
        <tt:Date>
          <tt:Year>${d.getUTCFullYear()}</tt:Year>
          <tt:Month>${d.getUTCMonth() + 1}</tt:Month>
          <tt:Day>${d.getUTCDate()}</tt:Day>
        </tt:Date>
      </tds:UTCDateTime>
    `
  }

  const body = `<tds:SetSystemDateAndTime>
    <tds:DateTimeType>${dateTimeType}</tds:DateTimeType>
    <tds:DaylightSavings>${daylightSavings}</tds:DaylightSavings>
    <tds:TimeZone>
      <tt:TZ>${timezone}</tt:TZ>
    </tds:TimeZone>
    ${utcDateTimeXml}
  </tds:SetSystemDateAndTime>`

  const envelope = createSOAPEnvelope(body)

  const response = await apiClient.post(ENDPOINTS.device, envelope)
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data)

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to set system date/time')
  }
}
