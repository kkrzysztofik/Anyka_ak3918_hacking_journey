/**
 * Imaging Service
 *
 * SOAP operations for imaging settings (brightness, contrast, saturation, IR cut filter, WDR, backlight compensation).
 */
import { ENDPOINTS, apiClient } from '@/services/api';
import { createSOAPEnvelope, parseSOAPResponse } from '@/services/soap/client';

export type IrCutFilterMode = 'ON' | 'OFF' | 'AUTO';
export type WideDynamicMode = 'ON' | 'OFF';
export type BacklightCompensationMode = 'ON' | 'OFF';

export interface ImagingSettings {
  brightness: number;
  contrast: number;
  saturation: number;
  sharpness: number;
  irCutFilter?: IrCutFilterMode;
  wideDynamicRange?: {
    mode: WideDynamicMode;
    level: number;
  };
  backlightCompensation?: {
    mode: BacklightCompensationMode;
    level: number;
  };
}

export interface ImagingOptions {
  brightness?: { min: number; max: number };
  contrast?: { min: number; max: number };
  saturation?: { min: number; max: number };
  sharpness?: { min: number; max: number };
  irCutFilterModes?: IrCutFilterMode[];
  wideDynamicRange?: {
    modes: WideDynamicMode[];
    level?: { min: number; max: number };
  };
  backlightCompensation?: {
    modes: BacklightCompensationMode[];
    level?: { min: number; max: number };
  };
}

/**
 * Helper function to parse IR Cut Filter mode
 */
function parseIrCutFilter(irCutFilter: unknown): IrCutFilterMode | undefined {
  if (!irCutFilter) {
    return undefined;
  }
  const mode = typeof irCutFilter === 'string' ? irCutFilter : String(irCutFilter);
  if (mode === 'ON' || mode === 'OFF' || mode === 'AUTO') {
    return mode as IrCutFilterMode;
  }
  return undefined;
}

/**
 * Helper function to parse Wide Dynamic Range settings
 */
function parseWideDynamicRangeSettings(
  wdr: Record<string, unknown> | undefined,
): ImagingSettings['wideDynamicRange'] {
  if (!wdr) {
    return undefined;
  }
  const modeValue = wdr.Mode ?? 'OFF';
  const mode = typeof modeValue === 'string' ? modeValue : String(modeValue);
  const level = Number(wdr.Level ?? 50);
  if (mode === 'ON' || mode === 'OFF') {
    return {
      mode: mode as WideDynamicMode,
      level,
    };
  }
  return undefined;
}

/**
 * Helper function to parse Backlight Compensation settings
 */
function parseBacklightCompensationSettings(
  backlight: Record<string, unknown> | undefined,
): ImagingSettings['backlightCompensation'] {
  if (!backlight) {
    return undefined;
  }
  const modeValue = backlight.Mode ?? 'OFF';
  const mode = typeof modeValue === 'string' ? modeValue : String(modeValue);
  const level = Number(backlight.Level ?? 50);
  if (mode === 'ON' || mode === 'OFF') {
    return {
      mode: mode as BacklightCompensationMode,
      level,
    };
  }
  return undefined;
}

/**
 * Get imaging settings for a video source
 */
export async function getImagingSettings(
  videoSourceToken: string = 'VideoSource_1',
): Promise<ImagingSettings> {
  const body = `<timg:GetImagingSettings>
    <timg:VideoSourceToken>${videoSourceToken}</timg:VideoSourceToken>
  </timg:GetImagingSettings>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.imaging, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get imaging settings');
  }

  const data = parsed.data?.GetImagingSettingsResponse as Record<string, unknown> | undefined;
  const settings = data?.ImagingSettings as Record<string, unknown> | undefined;

  // Handle case where ImagingSettings exists but is empty (will use defaults)
  if (settings === undefined || settings === null) {
    throw new Error('Invalid response: missing ImagingSettings');
  }

  const result: ImagingSettings = {
    brightness: Number(settings?.Brightness ?? 50),
    contrast: Number(settings?.Contrast ?? 50),
    saturation: Number(settings?.ColorSaturation ?? 50),
    sharpness: Number(settings?.Sharpness ?? 50),
  };

  // Parse optional settings using helper functions
  result.irCutFilter = parseIrCutFilter(settings?.IrCutFilter);
  result.wideDynamicRange = parseWideDynamicRangeSettings(
    settings?.WideDynamicRange as Record<string, unknown> | undefined,
  );
  result.backlightCompensation = parseBacklightCompensationSettings(
    settings?.BacklightCompensation as Record<string, unknown> | undefined,
  );

  return result;
}

/**
 * Set imaging settings for a video source
 */
export async function setImagingSettings(
  settings: Partial<ImagingSettings>,
  videoSourceToken: string = 'VideoSource_1',
): Promise<void> {
  const settingsXmlParts: string[] = [];

  if (settings.brightness !== undefined) {
    settingsXmlParts.push(`<tt:Brightness>${settings.brightness}</tt:Brightness>`);
  }
  if (settings.contrast !== undefined) {
    settingsXmlParts.push(`<tt:Contrast>${settings.contrast}</tt:Contrast>`);
  }
  if (settings.saturation !== undefined) {
    settingsXmlParts.push(`<tt:ColorSaturation>${settings.saturation}</tt:ColorSaturation>`);
  }
  if (settings.sharpness !== undefined) {
    settingsXmlParts.push(`<tt:Sharpness>${settings.sharpness}</tt:Sharpness>`);
  }
  if (settings.irCutFilter !== undefined) {
    settingsXmlParts.push(`<tt:IrCutFilter>${settings.irCutFilter}</tt:IrCutFilter>`);
  }
  if (settings.wideDynamicRange !== undefined) {
    settingsXmlParts.push(
      `<tt:WideDynamicRange>
        <tt:Mode>${settings.wideDynamicRange.mode}</tt:Mode>
        <tt:Level>${settings.wideDynamicRange.level}</tt:Level>
      </tt:WideDynamicRange>`,
    );
  }
  if (settings.backlightCompensation !== undefined) {
    settingsXmlParts.push(
      `<tt:BacklightCompensation>
        <tt:Mode>${settings.backlightCompensation.mode}</tt:Mode>
        <tt:Level>${settings.backlightCompensation.level}</tt:Level>
      </tt:BacklightCompensation>`,
    );
  }

  const settingsXml = settingsXmlParts.join('\n      ');

  const body = `<timg:SetImagingSettings>
    <timg:VideoSourceToken>${videoSourceToken}</timg:VideoSourceToken>
    <timg:ImagingSettings>
      ${settingsXml}
    </timg:ImagingSettings>
  </timg:SetImagingSettings>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.imaging, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to set imaging settings');
  }
}

/**
 * Helper function to parse a range from options
 */
function parseRange(
  rangeData: Record<string, unknown> | undefined,
): { min: number; max: number } | undefined {
  if (!rangeData) {
    return undefined;
  }
  return {
    min: Number(rangeData.Min ?? 0),
    max: Number(rangeData.Max ?? 100),
  };
}

/**
 * Helper function to parse level range
 */
function parseLevelRange(
  level: Record<string, unknown> | undefined,
): { min: number; max: number } | undefined {
  if (!level) {
    return undefined;
  }
  return {
    min: Number(level.Min ?? 0),
    max: Number(level.Max ?? 100),
  };
}

/**
 * Helper function to parse Wide Dynamic Range options
 */
function parseWideDynamicRange(
  wdr: Record<string, unknown> | undefined,
): ImagingOptions['wideDynamicRange'] {
  if (!wdr) {
    return undefined;
  }
  const modes = wdr.Mode;
  const level = wdr.Level as Record<string, unknown> | undefined;
  return {
    modes: Array.isArray(modes)
      ? modes.map(String).filter((m): m is WideDynamicMode => m === 'ON' || m === 'OFF')
      : [],
    level: parseLevelRange(level),
  };
}

/**
 * Helper function to parse Backlight Compensation options
 */
function parseBacklightCompensation(
  backlight: Record<string, unknown> | undefined,
): ImagingOptions['backlightCompensation'] {
  if (!backlight) {
    return undefined;
  }
  const modes = backlight.Mode;
  const level = backlight.Level as Record<string, unknown> | undefined;
  return {
    modes: Array.isArray(modes)
      ? modes.map(String).filter((m): m is BacklightCompensationMode => m === 'ON' || m === 'OFF')
      : [],
    level: parseLevelRange(level),
  };
}

/**
 * Get imaging options (valid ranges and supported modes) for a video source
 */
export async function getImagingOptions(
  videoSourceToken: string = 'VideoSource_1',
): Promise<ImagingOptions> {
  const body = `<timg:GetOptions>
    <timg:VideoSourceToken>${videoSourceToken}</timg:VideoSourceToken>
  </timg:GetOptions>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.imaging, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get imaging options');
  }

  const data = parsed.data?.GetOptionsResponse as Record<string, unknown> | undefined;
  const options = data?.ImagingOptions as Record<string, unknown> | undefined;

  if (!options) {
    throw new Error('Invalid response: missing ImagingOptions');
  }

  const result: ImagingOptions = {};

  // Parse brightness range
  result.brightness = parseRange(options.Brightness as Record<string, unknown> | undefined);

  // Parse contrast range
  result.contrast = parseRange(options.Contrast as Record<string, unknown> | undefined);

  // Parse saturation range
  result.saturation = parseRange(options.ColorSaturation as Record<string, unknown> | undefined);

  // Parse sharpness range
  result.sharpness = parseRange(options.Sharpness as Record<string, unknown> | undefined);

  // Parse IR Cut Filter modes
  const irCutFilterModes = options.IrCutFilterModes;
  if (Array.isArray(irCutFilterModes)) {
    result.irCutFilterModes = irCutFilterModes
      .map(String)
      .filter((m): m is IrCutFilterMode => m === 'ON' || m === 'OFF' || m === 'AUTO');
  }

  // Parse Wide Dynamic Range options
  result.wideDynamicRange = parseWideDynamicRange(
    options.WideDynamicRange as Record<string, unknown> | undefined,
  );

  // Parse Backlight Compensation options
  result.backlightCompensation = parseBacklightCompensation(
    options.BacklightCompensation as Record<string, unknown> | undefined,
  );

  return result;
}
