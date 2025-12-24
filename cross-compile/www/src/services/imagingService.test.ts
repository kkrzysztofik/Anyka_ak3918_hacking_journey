/**
 * Imaging Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import {
  getImagingOptions,
  getImagingSettings,
  setImagingSettings,
} from '@/services/imagingService';
import { createMockSOAPFaultResponse, createMockSOAPResponse } from '@/test/utils';

// Mock the api module
vi.mock('@/services/api', () => ({
  apiClient: {
    post: vi.fn(),
  },
  ENDPOINTS: {
    imaging: '/onvif/imaging_service',
  },
}));

describe('imagingService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getImagingSettings', () => {
    it('should parse basic imaging settings correctly', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetImagingSettingsResponse>
          <ImagingSettings>
            <Brightness>75</Brightness>
            <Contrast>60</Contrast>
            <ColorSaturation>55</ColorSaturation>
            <Sharpness>70</Sharpness>
          </ImagingSettings>
        </GetImagingSettingsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getImagingSettings();

      expect(result.brightness).toBe(75);
      expect(result.contrast).toBe(60);
      expect(result.saturation).toBe(55);
      expect(result.sharpness).toBe(70);
    });

    it('should parse IR cut filter mode', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetImagingSettingsResponse>
          <ImagingSettings>
            <Brightness>50</Brightness>
            <Contrast>50</Contrast>
            <ColorSaturation>50</ColorSaturation>
            <Sharpness>50</Sharpness>
            <IrCutFilter>AUTO</IrCutFilter>
          </ImagingSettings>
        </GetImagingSettingsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getImagingSettings();

      expect(result.irCutFilter).toBe('AUTO');
    });

    it('should parse wide dynamic range settings', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetImagingSettingsResponse>
          <ImagingSettings>
            <Brightness>50</Brightness>
            <Contrast>50</Contrast>
            <ColorSaturation>50</ColorSaturation>
            <Sharpness>50</Sharpness>
            <WideDynamicRange>
              <Mode>ON</Mode>
              <Level>75</Level>
            </WideDynamicRange>
          </ImagingSettings>
        </GetImagingSettingsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getImagingSettings();

      expect(result.wideDynamicRange).toEqual({
        mode: 'ON',
        level: 75,
      });
    });

    it('should parse backlight compensation settings', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetImagingSettingsResponse>
          <ImagingSettings>
            <Brightness>50</Brightness>
            <Contrast>50</Contrast>
            <ColorSaturation>50</ColorSaturation>
            <Sharpness>50</Sharpness>
            <BacklightCompensation>
              <Mode>ON</Mode>
              <Level>80</Level>
            </BacklightCompensation>
          </ImagingSettings>
        </GetImagingSettingsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getImagingSettings();

      expect(result.backlightCompensation).toEqual({
        mode: 'ON',
        level: 80,
      });
    });

    it('should use default values when settings are missing', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetImagingSettingsResponse>
          <ImagingSettings />
        </GetImagingSettingsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getImagingSettings();

      expect(result.brightness).toBe(50);
      expect(result.contrast).toBe(50);
      expect(result.saturation).toBe(50);
      expect(result.sharpness).toBe(50);
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Operation failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getImagingSettings()).rejects.toThrow();
    });

    it('should throw on missing ImagingSettings', async () => {
      const mockResponse = createMockSOAPResponse('<GetImagingSettingsResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getImagingSettings()).rejects.toThrow('Invalid response');
    });
  });

  describe('setImagingSettings', () => {
    it('should set basic imaging settings', async () => {
      const mockResponse = createMockSOAPResponse('<SetImagingSettingsResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setImagingSettings({
        brightness: 80,
        contrast: 70,
        saturation: 60,
        sharpness: 75,
      });

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('Brightness');
      expect(callArgs[1]).toContain('80');
      expect(callArgs[1]).toContain('Contrast');
      expect(callArgs[1]).toContain('70');
    });

    it('should set IR cut filter', async () => {
      const mockResponse = createMockSOAPResponse('<SetImagingSettingsResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setImagingSettings({
        brightness: 50,
        contrast: 50,
        saturation: 50,
        sharpness: 50,
        irCutFilter: 'ON',
      });

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('IrCutFilter');
      expect(callArgs[1]).toContain('ON');
    });

    it('should set wide dynamic range', async () => {
      const mockResponse = createMockSOAPResponse('<SetImagingSettingsResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setImagingSettings({
        brightness: 50,
        contrast: 50,
        saturation: 50,
        sharpness: 50,
        wideDynamicRange: {
          mode: 'ON',
          level: 85,
        },
      });

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('WideDynamicRange');
      expect(callArgs[1]).toContain('ON');
      expect(callArgs[1]).toContain('85');
    });

    it('should set backlight compensation', async () => {
      const mockResponse = createMockSOAPResponse('<SetImagingSettingsResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setImagingSettings({
        brightness: 50,
        contrast: 50,
        saturation: 50,
        sharpness: 50,
        backlightCompensation: {
          mode: 'ON',
          level: 90,
        },
      });

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('BacklightCompensation');
      expect(callArgs[1]).toContain('ON');
      expect(callArgs[1]).toContain('90');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Set failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(
        setImagingSettings({
          brightness: 50,
          contrast: 50,
          saturation: 50,
          sharpness: 50,
        }),
      ).rejects.toThrow();
    });
  });

  describe('getImagingOptions', () => {
    it('should parse imaging options correctly', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetOptionsResponse>
          <ImagingOptions>
            <Brightness>
              <Min>0</Min>
              <Max>100</Max>
            </Brightness>
            <Contrast>
              <Min>0</Min>
              <Max>100</Max>
            </Contrast>
            <ColorSaturation>
              <Min>0</Min>
              <Max>100</Max>
            </ColorSaturation>
            <Sharpness>
              <Min>0</Min>
              <Max>100</Max>
            </Sharpness>
            <IrCutFilterModes>ON</IrCutFilterModes>
            <IrCutFilterModes>OFF</IrCutFilterModes>
            <IrCutFilterModes>AUTO</IrCutFilterModes>
            <WideDynamicRange>
              <Mode>ON</Mode>
              <Mode>OFF</Mode>
              <Level>
                <Min>0</Min>
                <Max>100</Max>
              </Level>
            </WideDynamicRange>
            <BacklightCompensation>
              <Mode>ON</Mode>
              <Mode>OFF</Mode>
              <Level>
                <Min>0</Min>
                <Max>100</Max>
              </Level>
            </BacklightCompensation>
          </ImagingOptions>
        </GetOptionsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getImagingOptions();

      expect(result.brightness).toEqual({ min: 0, max: 100 });
      expect(result.contrast).toEqual({ min: 0, max: 100 });
      expect(result.saturation).toEqual({ min: 0, max: 100 });
      expect(result.sharpness).toEqual({ min: 0, max: 100 });
      expect(result.irCutFilterModes).toEqual(['ON', 'OFF', 'AUTO']);
      expect(result.wideDynamicRange?.modes).toEqual(['ON', 'OFF']);
      expect(result.wideDynamicRange?.level).toEqual({ min: 0, max: 100 });
      expect(result.backlightCompensation?.modes).toEqual(['ON', 'OFF']);
      expect(result.backlightCompensation?.level).toEqual({ min: 0, max: 100 });
    });

    it('should handle missing optional fields', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetOptionsResponse>
          <ImagingOptions>
            <Brightness>
              <Min>0</Min>
              <Max>100</Max>
            </Brightness>
          </ImagingOptions>
        </GetOptionsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getImagingOptions();

      expect(result.brightness).toEqual({ min: 0, max: 100 });
      expect(result.contrast).toBeUndefined();
      expect(result.wideDynamicRange).toBeUndefined();
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Get options failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getImagingOptions()).rejects.toThrow();
    });

    it('should throw on missing ImagingOptions', async () => {
      const mockResponse = createMockSOAPResponse('<GetOptionsResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getImagingOptions()).rejects.toThrow('Invalid response');
    });
  });
});
