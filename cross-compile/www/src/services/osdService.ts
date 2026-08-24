/**
 * OSD Service — ONVIF Media GetOSDs / GetOSD / SetOSD.
 *
 * Two fixed tokens: osd_name (Plain text) and osd_datetime (DateAndTime).
 * Colour and alpha are device-global.
 */
import { ENDPOINTS } from '@/services/api';
import { escapeXml, soapRequest } from '@/services/soap/client';
import { safeString } from '@/utils/safeString';

export const OSD_TOKEN_NAME = 'osd_name';
export const OSD_TOKEN_DATETIME = 'osd_datetime';

export type OsdCorner = 'UpperLeft' | 'UpperRight' | 'LowerLeft' | 'LowerRight';

export type OsdDateFormat = 'yyyy-MM-dd' | 'dd/MM/yyyy' | 'MM/dd/yyyy';
export type OsdTimeFormat = 'HH:mm:ss' | 'hh:mm:ss tt';

export interface OsdNameSettings {
  token: typeof OSD_TOKEN_NAME;
  position: OsdCorner;
  text: string;
  videoSourceToken: string;
}

export interface OsdDateTimeSettings {
  token: typeof OSD_TOKEN_DATETIME;
  position: OsdCorner;
  dateFormat: OsdDateFormat;
  timeFormat: OsdTimeFormat;
  videoSourceToken: string;
}

export interface OsdAppearance {
  /** Palette index 0..=15. */
  color: number;
  /** Opacity 1..=100. */
  alpha: number;
}

export interface OsdSettings {
  name: OsdNameSettings;
  datetime: OsdDateTimeSettings;
  appearance: OsdAppearance;
}

const CORNERS: OsdCorner[] = ['UpperLeft', 'UpperRight', 'LowerLeft', 'LowerRight'];

function parseCorner(value: unknown): OsdCorner {
  const s = safeString(value, 'UpperLeft');
  return (CORNERS as string[]).includes(s) ? (s as OsdCorner) : 'UpperLeft';
}

function parseDateFormat(value: unknown): OsdDateFormat {
  const s = safeString(value, 'yyyy-MM-dd');
  if (s === 'dd/MM/yyyy' || s === 'MM/dd/yyyy' || s === 'yyyy-MM-dd') {
    return s;
  }
  return 'yyyy-MM-dd';
}

function parseTimeFormat(value: unknown): OsdTimeFormat {
  const s = safeString(value, 'HH:mm:ss');
  if (s === 'hh:mm:ss tt' || s === 'HH:mm:ss') {
    return s;
  }
  return 'HH:mm:ss';
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return undefined;
}

function parseAppearance(text: Record<string, unknown> | undefined): OsdAppearance {
  const fontColor = asRecord(text?.FontColor);
  const color = asRecord(fontColor?.Color);
  const x = Number(color?.['@_X'] ?? 1 / 15);
  // Greyscale palette: index ≈ X * 15
  const colorIndex = Math.round(Math.min(1, Math.max(0, x)) * 15);
  const alphaRaw = Number(fontColor?.['@_Transparent'] ?? 80);
  const alpha = Number.isFinite(alphaRaw) ? Math.min(100, Math.max(1, alphaRaw)) : 80;
  return { color: colorIndex, alpha };
}

function parseOsdNode(node: Record<string, unknown>): {
  token: string;
  videoSourceToken: string;
  position: OsdCorner;
  text: Record<string, unknown> | undefined;
  appearance: OsdAppearance;
} {
  const position = asRecord(node.Position);
  const text = asRecord(node.TextString);
  return {
    token: safeString(node['@_token'], ''),
    videoSourceToken: safeString(node.VideoSourceConfigurationToken, 'VideoSourceToken'),
    position: parseCorner(position?.Type),
    text,
    appearance: parseAppearance(text),
  };
}

/** Reject non-ASCII before SetOSD — the camera font has no Latin diacritics. */
export function assertAsciiOsdText(text: string): void {
  if (text.length === 0) {
    return;
  }
  if (![...text].every((ch) => ch.charCodeAt(0) < 128)) {
    throw new Error(
      'OSD text must be ASCII: the camera font is GB2312 and has no glyph for non-ASCII characters',
    );
  }
}

/**
 * Fetch both fixed OSDs and derive device-global appearance from FontColor.
 */
export async function getOsdSettings(): Promise<OsdSettings> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.media,
    '<trt:GetOSDs />',
    'GetOSDsResponse',
  );

  const raw = data?.OSDs;
  const list = (Array.isArray(raw) ? raw : raw ? [raw] : []) as Record<string, unknown>[];

  let name: OsdNameSettings | undefined;
  let datetime: OsdDateTimeSettings | undefined;
  let appearance: OsdAppearance = { color: 1, alpha: 80 };

  for (const node of list) {
    const parsed = parseOsdNode(node);
    appearance = parsed.appearance;
    if (parsed.token === OSD_TOKEN_NAME) {
      name = {
        token: OSD_TOKEN_NAME,
        position: parsed.position,
        text: safeString(parsed.text?.PlainText, ''),
        videoSourceToken: parsed.videoSourceToken,
      };
    } else if (parsed.token === OSD_TOKEN_DATETIME) {
      datetime = {
        token: OSD_TOKEN_DATETIME,
        position: parsed.position,
        dateFormat: parseDateFormat(parsed.text?.DateFormat),
        timeFormat: parseTimeFormat(parsed.text?.TimeFormat),
        videoSourceToken: parsed.videoSourceToken,
      };
    }
  }

  if (!name || !datetime) {
    throw new Error('Invalid response: expected osd_name and osd_datetime');
  }

  return { name, datetime, appearance };
}

function colorChannelsXml(index: number): string {
  const t = (Math.min(15, Math.max(0, index)) / 15).toFixed(4);
  return `<tt:Color X="${t}" Y="${t}" Z="${t}" />`;
}

/**
 * Persist one OSD configuration (name or datetime) plus device-global style.
 */
export async function setOsd(settings: {
  token: string;
  videoSourceToken: string;
  position: OsdCorner;
  textType: 'Plain' | 'DateAndTime';
  plainText?: string;
  dateFormat?: OsdDateFormat;
  timeFormat?: OsdTimeFormat;
  color: number;
  alpha: number;
}): Promise<void> {
  if (settings.textType === 'Plain' && settings.plainText !== undefined) {
    assertAsciiOsdText(settings.plainText);
  }

  const textParts: string[] = [
    `<tt:Type>${settings.textType}</tt:Type>`,
    `<tt:FontSize>16</tt:FontSize>`,
  ];
  if (settings.textType === 'Plain') {
    textParts.push(`<tt:PlainText>${escapeXml(settings.plainText ?? '')}</tt:PlainText>`);
  } else {
    textParts.push(
      `<tt:DateFormat>${escapeXml(settings.dateFormat ?? 'yyyy-MM-dd')}</tt:DateFormat>`,
    );
    textParts.push(
      `<tt:TimeFormat>${escapeXml(settings.timeFormat ?? 'HH:mm:ss')}</tt:TimeFormat>`,
    );
  }
  textParts.push(
    `<tt:FontColor Transparent="${settings.alpha}">${colorChannelsXml(settings.color)}</tt:FontColor>`,
  );

  const body = `<trt:SetOSD>
    <trt:OSD token="${escapeXml(settings.token)}">
      <tt:VideoSourceConfigurationToken>${escapeXml(settings.videoSourceToken)}</tt:VideoSourceConfigurationToken>
      <tt:Type>Text</tt:Type>
      <tt:Position>
        <tt:Type>${settings.position}</tt:Type>
      </tt:Position>
      <tt:TextString>
        ${textParts.join('\n        ')}
      </tt:TextString>
    </trt:OSD>
  </trt:SetOSD>`;

  await soapRequest(ENDPOINTS.media, body, 'SetOSDResponse');
}
