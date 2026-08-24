/**
 * OSD Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import { assertAsciiOsdText, getOsdSettings, setOsd } from '@/services/osdService';
import { createMockSOAPFaultResponse, createMockSOAPResponse } from '@/test/utils';

vi.mock('@/services/api', () => ({
  apiClient: {
    post: vi.fn(),
  },
  ENDPOINTS: {
    media: '/onvif/media_service',
  },
}));

describe('osdService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('assertAsciiOsdText', () => {
    it('should accept ASCII text', () => {
      expect(() => assertAsciiOsdText('CAM1')).not.toThrow();
    });

    it('should reject non-ASCII text', () => {
      expect(() => assertAsciiOsdText('Ogród')).toThrow(/ASCII/);
    });
  });

  describe('getOsdSettings', () => {
    it('should parse both fixed OSD tokens', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetOSDsResponse>
          <OSDs token="osd_name">
            <VideoSourceConfigurationToken>VS0</VideoSourceConfigurationToken>
            <Type>Text</Type>
            <Position><Type>UpperLeft</Type></Position>
            <TextString>
              <Type>Plain</Type>
              <FontSize>16</FontSize>
              <PlainText>FRONT</PlainText>
              <FontColor Transparent="80"><Color X="0.0667" Y="0.0667" Z="0.0667" /></FontColor>
            </TextString>
          </OSDs>
          <OSDs token="osd_datetime">
            <VideoSourceConfigurationToken>VS0</VideoSourceConfigurationToken>
            <Type>Text</Type>
            <Position><Type>LowerRight</Type></Position>
            <TextString>
              <Type>DateAndTime</Type>
              <DateFormat>yyyy-MM-dd</DateFormat>
              <TimeFormat>HH:mm:ss</TimeFormat>
              <FontSize>16</FontSize>
              <FontColor Transparent="80"><Color X="0.0667" Y="0.0667" Z="0.0667" /></FontColor>
            </TextString>
          </OSDs>
        </GetOSDsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getOsdSettings();

      expect(result.name.text).toBe('FRONT');
      expect(result.name.position).toBe('UpperLeft');
      expect(result.datetime.position).toBe('LowerRight');
      expect(result.datetime.dateFormat).toBe('yyyy-MM-dd');
      expect(result.appearance.alpha).toBe(80);
    });

    it('should surface a SOAP fault as an Error', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce(
        createMockSOAPFaultResponse('ter:NotAuthorized', 'Not authorized'),
      );

      await expect(getOsdSettings()).rejects.toThrow(/Not authorized|Failed/);
    });
  });

  describe('setOsd', () => {
    it('should include the OSD token in the request body', async () => {
      vi.mocked(apiClient.post).mockResolvedValueOnce(createMockSOAPResponse('<SetOSDResponse />'));

      await setOsd({
        token: 'osd_name',
        videoSourceToken: 'VS0',
        position: 'LowerLeft',
        textType: 'Plain',
        plainText: 'CAM',
        color: 1,
        alpha: 80,
      });

      const body = String(vi.mocked(apiClient.post).mock.calls[0]?.[1] ?? '');
      expect(body).toContain('token="osd_name"');
      expect(body).toContain('LowerLeft');
      expect(body).toContain('CAM');
    });

    it('should reject non-ASCII plain text before the request', async () => {
      await expect(
        setOsd({
          token: 'osd_name',
          videoSourceToken: 'VS0',
          position: 'UpperLeft',
          textType: 'Plain',
          plainText: 'Ogród',
          color: 1,
          alpha: 80,
        }),
      ).rejects.toThrow(/ASCII/);

      expect(apiClient.post).not.toHaveBeenCalled();
    });
  });
});
