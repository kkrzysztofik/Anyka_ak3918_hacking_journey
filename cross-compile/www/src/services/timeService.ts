/**
 * Time Service
 *
 * SOAP operations for system date/time configuration.
 */
import { ENDPOINTS } from '@/services/api';
import { soapBodies, soapRequest } from '@/services/soap/client';
import { safeString } from '@/utils/safeString';

export type DateTimeType = 'NTP' | 'Manual';

export interface SystemDateTime {
  dateTimeType: DateTimeType;
  daylightSavings: boolean;
  timezone: string;
  utcDateTime: Date;
}

export interface DateTimeConfig {
  ntp: {
    enabled: boolean;
    fromDHCP: boolean;
  };
  timezone: string;
  datetime: Date;
}

/**
 * Get system date and time configuration
 */
export async function getSystemDateAndTime(): Promise<SystemDateTime> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    soapBodies.getSystemDateAndTime(),
    'GetSystemDateAndTimeResponse',
  );

  const sdt = data?.SystemDateAndTime as Record<string, unknown> | undefined;

  if (!sdt) {
    throw new Error('Invalid response: missing SystemDateAndTime');
  }

  const utcDateTime = sdt.UTCDateTime as Record<string, unknown> | undefined;
  const time = utcDateTime?.Time as Record<string, unknown> | undefined;
  const date = utcDateTime?.Date as Record<string, unknown> | undefined;
  const timezone = sdt.TimeZone as Record<string, unknown> | undefined;

  // Build Date object from response
  const year = Number(date?.Year || new Date().getFullYear());
  const month = Number(date?.Month || 1) - 1; // JS months are 0-indexed
  const day = Number(date?.Day || 1);
  const hour = Number(time?.Hour || 0);
  const minute = Number(time?.Minute || 0);
  const second = Number(time?.Second || 0);

  return {
    dateTimeType: safeString(sdt.DateTimeType, 'NTP') as DateTimeType,
    daylightSavings: sdt.DaylightSavings === true || sdt.DaylightSavings === 'true',
    timezone: safeString(timezone?.TZ, 'UTC'),
    utcDateTime: new Date(Date.UTC(year, month, day, hour, minute, second)),
  };
}

/**
 * Set system date and time configuration
 */
export async function setSystemDateAndTime(
  dateTimeType: DateTimeType,
  daylightSavings: boolean,
  timezone: string,
  manualDateTime?: Date,
): Promise<void> {
  let utcDateTimeXml = '';

  if (dateTimeType === 'Manual' && manualDateTime) {
    const d = manualDateTime;
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
    `;
  }

  const body = `<tds:SetSystemDateAndTime>
    <tds:DateTimeType>${dateTimeType}</tds:DateTimeType>
    <tds:DaylightSavings>${daylightSavings}</tds:DaylightSavings>
    <tds:TimeZone>
      <tt:TZ>${timezone}</tt:TZ>
    </tds:TimeZone>
    ${utcDateTimeXml}
  </tds:SetSystemDateAndTime>`;

  await soapRequest(ENDPOINTS.device, body);
}

/**
 * Get date time config (Adapter for TimePage)
 */
export async function getDateTime(): Promise<DateTimeConfig> {
  const sys = await getSystemDateAndTime();
  return {
    ntp: {
      enabled: sys.dateTimeType === 'NTP',
      fromDHCP: true, // Stub
    },
    timezone: sys.timezone,
    datetime: sys.utcDateTime,
  };
}

/**
 * Set NTP mode
 */
export async function setNTP(_fromDHCP: boolean): Promise<void> {
  // We preserve current timezone and assume NTP mode
  const current = await getSystemDateAndTime();
  await setSystemDateAndTime('NTP', current.daylightSavings, current.timezone);
  // Note: fromDHCP logic might require lower-level network interface changes (DNS/NTP from DHCP)
  // which might be handled in Network settings, but here we just enable NTP mode.
}

/**
 * Set DateTime manual
 */
export async function setDateTime(
  mode: 'manual',
  isoDate: string,
  timezone: string,
): Promise<void> {
  await setSystemDateAndTime('Manual', false, timezone, new Date(isoDate));
}
