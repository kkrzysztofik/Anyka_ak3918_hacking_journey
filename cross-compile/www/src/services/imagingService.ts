/**
 * Imaging Service
 *
 * SOAP operations for imaging settings (brightness, contrast, saturation).
 */
import { ENDPOINTS, apiClient } from '@/services/api';
import { createSOAPEnvelope, parseSOAPResponse } from '@/services/soap/client';

export interface ImagingSettings {
  brightness: number;
  contrast: number;
  saturation: number;
  sharpness: number;
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

  return {
    brightness: Number(settings?.Brightness ?? 50),
    contrast: Number(settings?.Contrast ?? 50),
    saturation: Number(settings?.ColorSaturation ?? 50),
    sharpness: Number(settings?.Sharpness ?? 50),
  };
}

/**
 * Set imaging settings for a video source
 */
export async function setImagingSettings(
  settings: Partial<ImagingSettings>,
  videoSourceToken: string = 'VideoSource_1',
): Promise<void> {
  const settingsXml = [
    settings.brightness !== undefined
      ? `<tt:Brightness>${settings.brightness}</tt:Brightness>`
      : '',
    settings.contrast !== undefined ? `<tt:Contrast>${settings.contrast}</tt:Contrast>` : '',
    settings.saturation !== undefined
      ? `<tt:ColorSaturation>${settings.saturation}</tt:ColorSaturation>`
      : '',
    settings.sharpness !== undefined ? `<tt:Sharpness>${settings.sharpness}</tt:Sharpness>` : '',
  ]
    .filter(Boolean)
    .join('\n      ');

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
