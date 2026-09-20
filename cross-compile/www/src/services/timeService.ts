/**
 * Time Service
 *
 * SOAP operations for system date/time configuration.
 */
import { ENDPOINTS } from '@/services/api';
import { escapeXml, soapBodies, soapRequest } from '@/services/soap/client';
import { safeString } from '@/utils/safeString';

export type DateTimeType = 'NTP' | 'Manual';

export interface SystemDateTime {
  dateTimeType: DateTimeType;
  daylightSavings: boolean;
  timezone: string;
  utcDateTime: Date;
  /** Camera-local time as the camera computes it; null when absent. */
  localDateTime: Date | null;
}

export interface DateTimeConfig {
  ntp: {
    enabled: boolean;
  };
  daylightSavings: boolean;
  timezone: string;
  utcDateTime: Date;
  /** Camera-local time; absent on firmware that does not report it. */
  localDateTime?: Date | null;
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
  const timezone = sdt.TimeZone as Record<string, unknown> | undefined;

  const toJsDate = (block: Record<string, unknown> | undefined): Date | null => {
    const t = block?.Time as Record<string, unknown> | undefined;
    const d = block?.Date as Record<string, unknown> | undefined;
    if (!t && !d) return null;
    // Partial blocks fall back the same way the old inline parser did.
    return new Date(
      Date.UTC(
        Number(d?.Year || new Date().getFullYear()),
        Number(d?.Month || 1) - 1,
        Number(d?.Day || 1),
        Number(t?.Hour || 0),
        Number(t?.Minute || 0),
        Number(t?.Second || 0),
      ),
    );
  };

  return {
    dateTimeType: safeString(sdt.DateTimeType, 'NTP') as DateTimeType,
    daylightSavings: sdt.DaylightSavings === true || sdt.DaylightSavings === 'true',
    timezone: safeString(timezone?.TZ, 'UTC'),
    utcDateTime: toJsDate(utcDateTime) ?? new Date(),
    localDateTime: toJsDate(sdt.LocalDateTime as Record<string, unknown> | undefined),
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
  const escapedDateTimeType = escapeXml(dateTimeType);
  const escapedTimezone = escapeXml(timezone);

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
    <tds:DateTimeType>${escapedDateTimeType}</tds:DateTimeType>
    <tds:DaylightSavings>${daylightSavings}</tds:DaylightSavings>
    <tds:TimeZone>
      <tt:TZ>${escapedTimezone}</tt:TZ>
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
    },
    daylightSavings: sys.daylightSavings,
    timezone: sys.timezone,
    utcDateTime: sys.utcDateTime,
    localDateTime: sys.localDateTime,
  };
}

/**
 * Get the NTP server list reported by the camera.
 *
 * fast-xml-parser collapses a one-element NTPManual list to a single object;
 * tolerate both shapes.
 */
export async function getNtp(): Promise<string[]> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    '<tds:GetNTP />',
    'GetNTPResponse',
  );

  const raw = (data?.NTPInformation as Record<string, unknown> | undefined)?.NTPManual;
  // fast-xml-parser collapses a one-element list to a bare object.
  const present = raw ?? [];
  const entries = Array.isArray(present) ? present : [present];
  return entries
    .map((entry) => {
      const e = entry as Record<string, unknown>;
      const value = e.DNSname ?? e.IPv4Address ?? e.IPv6Address;
      return typeof value === 'string' ? value : '';
    })
    .filter((s) => s.length > 0);
}

// Octet-accurate on purpose: `\d{1,3}` alone accepts 999.999.999.999 and
// would ship it as <tt:IPv4Address>, which the camera rejects, instead of
// letting it through as a (also invalid, but correctly typed) DNS name.
const IPV4_OCTET = String.raw`(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)`;
const IPV4_RE = new RegExp(String.raw`^${IPV4_OCTET}(?:\.${IPV4_OCTET}){3}$`);

/**
 * Set the NTP server list. One `<tds:NTPManual>` per server; an IPv4 literal
 * goes out as `<tt:IPv4Address>`, everything else as `<tt:DNSname>`.
 */
export async function setNtp(servers: string[]): Promise<void> {
  const manual = servers
    .map((s) => {
      const escaped = escapeXml(s);
      return IPV4_RE.test(s)
        ? `<tds:NTPManual><tt:Type>IPv4</tt:Type><tt:IPv4Address>${escaped}</tt:IPv4Address></tds:NTPManual>`
        : `<tds:NTPManual><tt:Type>DNS</tt:Type><tt:DNSname>${escaped}</tt:DNSname></tds:NTPManual>`;
    })
    .join('');

  const body = `<tds:SetNTP><tds:FromDHCP>false</tds:FromDHCP>${manual}</tds:SetNTP>`;
  await soapRequest(ENDPOINTS.device, body, 'SetNTPResponse');
}

/**
 * Set DateTime manual
 */
export async function setDateTime(
  isoDate: string,
  timezone: string,
  daylightSavings: boolean,
): Promise<void> {
  await setSystemDateAndTime('Manual', daylightSavings, timezone, new Date(isoDate));
}
