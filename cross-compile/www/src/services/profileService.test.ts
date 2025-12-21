/**
 * Profile Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import {
  getVideoEncoderConfiguration,
  getVideoEncoderConfigurationOptions,
  getVideoEncoderConfigurations,
  setVideoEncoderConfiguration,
} from '@/services/profileService';

// Mock the api module
vi.mock('@/services/api', () => ({
  apiClient: {
    post: vi.fn(),
  },
  ENDPOINTS: {
    media: '/onvif/media_service',
  },
}));

describe('profileService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getVideoEncoderConfigurations', () => {
    it('should parse video encoder configurations correctly', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <GetVideoEncoderConfigurationsResponse>
                <Configurations token="encoder_1">
                  <Name>MainStream</Name>
                  <Encoding>H264</Encoding>
                  <Resolution>
                    <Width>1920</Width>
                    <Height>1080</Height>
                  </Resolution>
                  <Quality>80</Quality>
                  <RateControl>
                    <FrameRateLimit>30</FrameRateLimit>
                    <EncodingInterval>1</EncodingInterval>
                    <BitrateLimit>4000</BitrateLimit>
                  </RateControl>
                  <H264>
                    <GovLength>30</GovLength>
                    <H264Profile>Main</H264Profile>
                  </H264>
                  <SessionTimeout>PT60S</SessionTimeout>
                </Configurations>
              </GetVideoEncoderConfigurationsResponse>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfigurations();

      expect(result).toHaveLength(1);
      expect(result[0].token).toBe('encoder_1');
      expect(result[0].name).toBe('MainStream');
      expect(result[0].encoding).toBe('H264');
      expect(result[0].resolution).toEqual({ width: 1920, height: 1080 });
      expect(result[0].quality).toBe(80);
      expect(result[0].rateControl).toEqual({
        frameRateLimit: 30,
        encodingInterval: 1,
        bitrateLimit: 4000,
      });
      expect(result[0].h264).toEqual({
        govLength: 30,
        h264Profile: 'Main',
      });
    });

    it('should handle multiple configurations', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <GetVideoEncoderConfigurationsResponse>
                <Configurations token="encoder_1">
                  <Name>MainStream</Name>
                  <Encoding>H264</Encoding>
                  <Resolution><Width>1920</Width><Height>1080</Height></Resolution>
                  <Quality>80</Quality>
                  <SessionTimeout>PT60S</SessionTimeout>
                </Configurations>
                <Configurations token="encoder_2">
                  <Name>SubStream</Name>
                  <Encoding>H264</Encoding>
                  <Resolution><Width>640</Width><Height>480</Height></Resolution>
                  <Quality>60</Quality>
                  <SessionTimeout>PT60S</SessionTimeout>
                </Configurations>
              </GetVideoEncoderConfigurationsResponse>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfigurations();

      expect(result).toHaveLength(2);
      expect(result[0].name).toBe('MainStream');
      expect(result[1].name).toBe('SubStream');
    });

    it('should return empty array when no configurations', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <GetVideoEncoderConfigurationsResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfigurations();

      expect(result).toEqual([]);
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <soap:Fault>
                <soap:Code><soap:Value>soap:Sender</soap:Value></soap:Code>
                <soap:Reason><soap:Text>Operation failed</soap:Text></soap:Reason>
              </soap:Fault>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getVideoEncoderConfigurations()).rejects.toThrow();
    });
  });

  describe('getVideoEncoderConfiguration', () => {
    it('should parse single video encoder configuration', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <GetVideoEncoderConfigurationResponse>
                <Configuration token="encoder_1">
                  <Name>MainStream</Name>
                  <Encoding>H264</Encoding>
                  <Resolution>
                    <Width>1920</Width>
                    <Height>1080</Height>
                  </Resolution>
                  <Quality>80</Quality>
                  <RateControl>
                    <FrameRateLimit>30</FrameRateLimit>
                    <EncodingInterval>1</EncodingInterval>
                    <BitrateLimit>4000</BitrateLimit>
                  </RateControl>
                  <H264>
                    <GovLength>30</GovLength>
                    <H264Profile>Main</H264Profile>
                  </H264>
                  <SessionTimeout>PT60S</SessionTimeout>
                </Configuration>
              </GetVideoEncoderConfigurationResponse>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfiguration('encoder_1');

      expect(result).not.toBeNull();
      expect(result?.token).toBe('encoder_1');
      expect(result?.name).toBe('MainStream');
    });

    it('should return null on SOAP fault', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <soap:Fault>
                <soap:Code><soap:Value>soap:Sender</soap:Value></soap:Code>
                <soap:Reason><soap:Text>Not found</soap:Text></soap:Reason>
              </soap:Fault>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfiguration('invalid');

      expect(result).toBeNull();
    });

    it('should return null on missing configuration', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <GetVideoEncoderConfigurationResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfiguration('invalid');

      expect(result).toBeNull();
    });
  });

  describe('setVideoEncoderConfiguration', () => {
    it('should set video encoder configuration', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <SetVideoEncoderConfigurationResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const config = {
        token: 'encoder_1',
        name: 'MainStream',
        encoding: 'H264' as const,
        resolution: { width: 1920, height: 1080 },
        quality: 80,
        rateControl: {
          frameRateLimit: 30,
          encodingInterval: 1,
          bitrateLimit: 4000,
        },
        h264: {
          govLength: 30,
          h264Profile: 'Main',
        },
        sessionTimeout: 'PT60S',
      };

      await setVideoEncoderConfiguration(config);

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('encoder_1');
      expect(callArgs[1]).toContain('MainStream');
      expect(callArgs[1]).toContain('1920');
      expect(callArgs[1]).toContain('1080');
      expect(callArgs[1]).toContain('80');
    });

    it('should include force persistence flag', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <SetVideoEncoderConfigurationResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const config = {
        token: 'encoder_1',
        name: 'MainStream',
        encoding: 'H264' as const,
        resolution: { width: 1920, height: 1080 },
        quality: 80,
        sessionTimeout: 'PT60S',
      };

      await setVideoEncoderConfiguration(config, true);

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('ForcePersistence');
      expect(callArgs[1]).toContain('true');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <soap:Fault>
                <soap:Code><soap:Value>soap:Sender</soap:Value></soap:Code>
                <soap:Reason><soap:Text>Set failed</soap:Text></soap:Reason>
              </soap:Fault>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const config = {
        token: 'encoder_1',
        name: 'MainStream',
        encoding: 'H264' as const,
        resolution: { width: 1920, height: 1080 },
        quality: 80,
        sessionTimeout: 'PT60S',
      };

      await expect(setVideoEncoderConfiguration(config)).rejects.toThrow();
    });
  });

  describe('getVideoEncoderConfigurationOptions', () => {
    it('should parse video encoder configuration options', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <GetVideoEncoderConfigurationOptionsResponse>
                <Options>
                  <QualityRange>
                    <Min>0</Min>
                    <Max>100</Max>
                  </QualityRange>
                  <H264>
                    <ResolutionsAvailable>
                      <Width>1920</Width>
                      <Height>1080</Height>
                    </ResolutionsAvailable>
                    <ResolutionsAvailable>
                      <Width>1280</Width>
                      <Height>720</Height>
                    </ResolutionsAvailable>
                    <FrameRateRange>
                      <Min>1</Min>
                      <Max>30</Max>
                    </FrameRateRange>
                    <EncodingIntervalRange>
                      <Min>1</Min>
                      <Max>30</Max>
                    </EncodingIntervalRange>
                    <BitrateRange>
                      <Min>64</Min>
                      <Max>8192</Max>
                    </BitrateRange>
                    <H264ProfilesSupported>Main</H264ProfilesSupported>
                    <H264ProfilesSupported>High</H264ProfilesSupported>
                    <GovLengthRange>
                      <Min>1</Min>
                      <Max>300</Max>
                    </GovLengthRange>
                  </H264>
                  <JPEG>
                    <ResolutionsAvailable>
                      <Width>1920</Width>
                      <Height>1080</Height>
                    </ResolutionsAvailable>
                    <FrameRateRange>
                      <Min>1</Min>
                      <Max>30</Max>
                    </FrameRateRange>
                    <EncodingIntervalRange>
                      <Min>1</Min>
                      <Max>30</Max>
                    </EncodingIntervalRange>
                  </JPEG>
                </Options>
              </GetVideoEncoderConfigurationOptionsResponse>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfigurationOptions();

      expect(result.qualityRange).toEqual({ min: 0, max: 100 });
      expect(result.h264?.resolutionsAvailable).toHaveLength(2);
      expect(result.h264?.resolutionsAvailable[0]).toEqual({ width: 1920, height: 1080 });
      expect(result.h264?.frameRateRange).toEqual({ min: 1, max: 30 });
      expect(result.h264?.bitrateRange).toEqual({ min: 64, max: 8192 });
      expect(result.h264?.h264ProfilesSupported).toEqual(['Main', 'High']);
      expect(result.h264?.govLengthRange).toEqual({ min: 1, max: 300 });
      expect(result.jpeg?.resolutionsAvailable).toHaveLength(1);
    });

    it('should handle optional configuration token and profile token', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <GetVideoEncoderConfigurationOptionsResponse>
                <Options>
                  <QualityRange><Min>0</Min><Max>100</Max></QualityRange>
                </Options>
              </GetVideoEncoderConfigurationOptionsResponse>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await getVideoEncoderConfigurationOptions('encoder_1', 'profile_1');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('encoder_1');
      expect(callArgs[1]).toContain('profile_1');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <soap:Fault>
                <soap:Code><soap:Value>soap:Sender</soap:Value></soap:Code>
                <soap:Reason><soap:Text>Get options failed</soap:Text></soap:Reason>
              </soap:Fault>
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getVideoEncoderConfigurationOptions()).rejects.toThrow();
    });

    it('should throw on missing Options', async () => {
      const mockResponse = {
        data: `<?xml version="1.0" encoding="UTF-8"?>
          <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
            <soap:Body>
              <GetVideoEncoderConfigurationOptionsResponse />
            </soap:Body>
          </soap:Envelope>`,
      };

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getVideoEncoderConfigurationOptions()).rejects.toThrow('Invalid response');
    });
  });
});
