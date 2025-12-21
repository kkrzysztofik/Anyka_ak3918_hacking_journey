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

  // Parse IR Cut Filter
  const irCutFilter = settings?.IrCutFilter;
  if (irCutFilter) {
    const mode = String(irCutFilter);
    if (mode === 'ON' || mode === 'OFF' || mode === 'AUTO') {
      result.irCutFilter = mode as IrCutFilterMode;
    }
  }

  // Parse Wide Dynamic Range
  const wdr = settings?.WideDynamicRange as Record<string, unknown> | undefined;
  if (wdr) {
    const mode = String(wdr.Mode || 'OFF');
    const level = Number(wdr.Level ?? 50);
    if (mode === 'ON' || mode === 'OFF') {
      result.wideDynamicRange = {
        mode: mode as WideDynamicMode,
        level,
      };
    }
  }

  // Parse Backlight Compensation
  const backlight = settings?.BacklightCompensation as Record<string, unknown> | undefined;
  if (backlight) {
    const mode = String(backlight.Mode || 'OFF');
    const level = Number(backlight.Level ?? 50);
    if (mode === 'ON' || mode === 'OFF') {
      result.backlightCompensation = {
        mode: mode as BacklightCompensationMode,
        level,
      };
    }
  }

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
  const brightness = options.Brightness as Record<string, unknown> | undefined;
  if (brightness) {
    result.brightness = {
      min: Number(brightness.Min ?? 0),
      max: Number(brightness.Max ?? 100),
    };
  }

  // Parse contrast range
  const contrast = options.Contrast as Record<string, unknown> | undefined;
  if (contrast) {
    result.contrast = {
      min: Number(contrast.Min ?? 0),
      max: Number(contrast.Max ?? 100),
    };
  }

  // Parse saturation range
  const saturation = options.ColorSaturation as Record<string, unknown> | undefined;
  if (saturation) {
    result.saturation = {
      min: Number(saturation.Min ?? 0),
      max: Number(saturation.Max ?? 100),
    };
  }

  // Parse sharpness range
  const sharpness = options.Sharpness as Record<string, unknown> | undefined;
  if (sharpness) {
    result.sharpness = {
      min: Number(sharpness.Min ?? 0),
      max: Number(sharpness.Max ?? 100),
    };
  }

  // Parse IR Cut Filter modes
  const irCutFilterModes = options.IrCutFilterModes;
  if (Array.isArray(irCutFilterModes)) {
    result.irCutFilterModes = irCutFilterModes
      .map((m) => String(m))
      .filter((m): m is IrCutFilterMode => m === 'ON' || m === 'OFF' || m === 'AUTO');
  }

  // Parse Wide Dynamic Range options
  const wdr = options.WideDynamicRange as Record<string, unknown> | undefined;
  if (wdr) {
    const modes = wdr.Mode;
    const level = wdr.Level as Record<string, unknown> | undefined;
    result.wideDynamicRange = {
      modes: Array.isArray(modes)
        ? modes.map((m) => String(m)).filter((m): m is WideDynamicMode => m === 'ON' || m === 'OFF')
        : [],
      level: level
        ? {
            min: Number(level.Min ?? 0),
            max: Number(level.Max ?? 100),
          }
        : undefined,
    };
  }

  // Parse Backlight Compensation options
  const backlight = options.BacklightCompensation as Record<string, unknown> | undefined;
  if (backlight) {
    const modes = backlight.Mode;
    const level = backlight.Level as Record<string, unknown> | undefined;
    result.backlightCompensation = {
      modes: Array.isArray(modes)
        ? modes
            .map((m) => String(m))
            .filter((m): m is BacklightCompensationMode => m === 'ON' || m === 'OFF')
        : [],
      level: level
        ? {
            min: Number(level.Min ?? 0),
            max: Number(level.Max ?? 100),
          }
        : undefined,
    };
  }

  return result;
}
