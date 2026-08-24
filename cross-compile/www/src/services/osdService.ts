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
  /** Derived from presence in GetOSDs — ONVIF has no Enabled flag. */
  enabled: boolean;
  position: OsdCorner;
  text: string;
  videoSourceToken: string;
}

export interface OsdDateTimeSettings {
  token: typeof OSD_TOKEN_DATETIME;
  /** Derived from presence in GetOSDs — ONVIF has no Enabled flag. */
  enabled: boolean;
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

/**
 * ONVIF colourspace the camera reports its palette in.
 *
 * An opaque identifier from the ONVIF spec, never dereferenced. It must match
 * byte-for-byte, so it cannot be "upgraded" to https.
 */
const COLORSPACE_YCBCR = 'http://www.onvif.org/ver10/colorspace/YCbCr'; // NOSONAR typescript:S5332 -- namespace id, never fetched

/**
 * The vendor's OSD colour table, verbatim from `def_color_tables[]` in
 * `ak_osd.h`, as `[Y, Cb, Cr]`.
 *
 * These are YCbCr, not RGB — index 1 (`0xff7f7f`) is white and index 2
 * (`0x007f7f`) is black, which only works with neutral chroma at 0x7f. The
 * camera only understands the index; the channels exist to draw a swatch.
 */
export const VENDOR_PALETTE: readonly (readonly [number, number, number])[] = [
  [0x00, 0x00, 0x00],
  [0xff, 0x7f, 0x7f],
  [0x00, 0x7f, 0x7f],
  [0x26, 0x6a, 0xc0],
  [0x71, 0x40, 0x8a],
  [0x4b, 0x55, 0x4a],
  [0x59, 0x95, 0x40],
  [0x0e, 0xc0, 0x75],
  [0x34, 0xaa, 0xb5],
  [0x78, 0x60, 0x85],
  [0x2c, 0x8a, 0xa0],
  [0x68, 0xd5, 0x35],
  [0x34, 0xaa, 0x5a],
  [0x43, 0xe9, 0xab],
  [0x4b, 0x55, 0xa5],
  [0x00, 0x80, 0x80],
];

/** BT.601 YCbCr → CSS rgb(), so a swatch matches what the video shows. */
export function paletteCss(index: number): string {
  const [y, cb, cr] = VENDOR_PALETTE[Math.min(15, Math.max(0, index))];
  const clamp = (v: number) => Math.round(Math.min(255, Math.max(0, v)));
  return `rgb(${clamp(y + 1.402 * (cr - 128))}, ${clamp(
    y - 0.344136 * (cb - 128) - 0.714136 * (cr - 128),
  )}, ${clamp(y + 1.772 * (cb - 128))})`;
}

function nearestPaletteIndex(y: number, cb: number, cr: number): number {
  if (!Number.isFinite(y) || !Number.isFinite(cb) || !Number.isFinite(cr)) {
    return 1;
  }
  let best = 0;
  let bestD = Infinity;
  VENDOR_PALETTE.forEach(([py, pcb, pcr], i) => {
    const d = (py - y) ** 2 + (pcb - cb) ** 2 + (pcr - cr) ** 2;
    if (d < bestD) {
      bestD = d;
      best = i;
    }
  });
  return best;
}

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
  const colorIndex = nearestPaletteIndex(
    Number(color?.['@_X'] ?? VENDOR_PALETTE[1][0]),
    Number(color?.['@_Y'] ?? VENDOR_PALETTE[1][1]),
    Number(color?.['@_Z'] ?? VENDOR_PALETTE[1][2]),
  );
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
  // Not a regex: an ASCII-range character class trips eslint's no-control-regex.
  if (![...text].every((ch) => ch.charCodeAt(0) < 128)) {
    throw new Error(
      'OSD text must be ASCII: the camera font is GB2312 and has no glyph for non-ASCII characters',
    );
  }
}

/**
 * Enable or disable one OSD.
 *
 * ONVIF has no Enabled flag on OSDConfiguration — presence in GetOSDs is the
 * enabled state, so CreateOSD/DeleteOSD are the toggle.
 */
export async function setOsdEnabled(
  settings: { token: string; videoSourceToken: string; position: OsdCorner },
  enabled: boolean,
): Promise<void> {
  if (!enabled) {
    await soapRequest(
      ENDPOINTS.media,
      `<trt:DeleteOSD><trt:OSDToken>${escapeXml(settings.token)}</trt:OSDToken></trt:DeleteOSD>`,
      'DeleteOSDResponse',
    );
    return;
  }
  const body = `<trt:CreateOSD>
    <trt:OSD token="${escapeXml(settings.token)}">
      <tt:VideoSourceConfigurationToken>${escapeXml(settings.videoSourceToken)}</tt:VideoSourceConfigurationToken>
      <tt:Type>Text</tt:Type>
      <tt:Position>
        <tt:Type>${settings.position}</tt:Type>
      </tt:Position>
    </trt:OSD>
  </trt:CreateOSD>`;
  await soapRequest(ENDPOINTS.media, body, 'CreateOSDResponse');
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

  // A disabled OSD is simply absent from GetOSDs, so start from disabled
  // defaults and let whatever came back switch it on.
  const videoSourceToken = safeString(list[0]?.VideoSourceConfigurationToken, 'VideoSourceToken');
  const name: OsdNameSettings = {
    token: OSD_TOKEN_NAME,
    enabled: false,
    position: 'UpperLeft',
    text: '',
    videoSourceToken,
  };
  const datetime: OsdDateTimeSettings = {
    token: OSD_TOKEN_DATETIME,
    enabled: false,
    position: 'LowerRight',
    dateFormat: 'yyyy-MM-dd',
    timeFormat: 'HH:mm:ss',
    videoSourceToken,
  };
  let appearance: OsdAppearance = { color: 1, alpha: 80 };

  for (const node of list) {
    const parsed = parseOsdNode(node);
    appearance = parsed.appearance;
    if (parsed.token === OSD_TOKEN_NAME) {
      name.enabled = true;
      name.position = parsed.position;
      name.text = safeString(parsed.text?.PlainText, '');
      name.videoSourceToken = parsed.videoSourceToken;
    } else if (parsed.token === OSD_TOKEN_DATETIME) {
      datetime.enabled = true;
      datetime.position = parsed.position;
      datetime.dateFormat = parseDateFormat(parsed.text?.DateFormat);
      datetime.timeFormat = parseTimeFormat(parsed.text?.TimeFormat);
      datetime.videoSourceToken = parsed.videoSourceToken;
    }
  }

  return { name, datetime, appearance };
}

function colorChannelsXml(index: number): string {
  const [y, cb, cr] = VENDOR_PALETTE[Math.min(15, Math.max(0, index))];
  return `<tt:Color X="${y}" Y="${cb}" Z="${cr}" Colorspace="${COLORSPACE_YCBCR}" />`;
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
