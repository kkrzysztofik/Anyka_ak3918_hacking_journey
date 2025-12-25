/**
 * Profile Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import {
  createProfile,
  deleteProfile,
  getProfile,
  getProfiles,
  getVideoEncoderConfiguration,
  getVideoEncoderConfigurationOptions,
  getVideoEncoderConfigurations,
  setVideoEncoderConfiguration,
} from '@/services/profileService';
import { createMockSOAPFaultResponse, createMockSOAPResponse } from '@/test/utils';

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
      const mockResponse = createMockSOAPResponse(`
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
      `);

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
      const mockResponse = createMockSOAPResponse(`
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
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfigurations();

      expect(result).toHaveLength(2);
      expect(result[0].name).toBe('MainStream');
      expect(result[1].name).toBe('SubStream');
    });

    it('should return empty array when no configurations', async () => {
      const mockResponse = createMockSOAPResponse('<GetVideoEncoderConfigurationsResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfigurations();

      expect(result).toEqual([]);
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Operation failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getVideoEncoderConfigurations()).rejects.toThrow();
    });
  });

  describe('getVideoEncoderConfiguration', () => {
    it('should parse single video encoder configuration', async () => {
      const mockResponse = createMockSOAPResponse(`
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
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfiguration('encoder_1');

      expect(result).not.toBeNull();
      expect(result?.token).toBe('encoder_1');
      expect(result?.name).toBe('MainStream');
    });

    it('should return null on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Not found');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfiguration('invalid');

      expect(result).toBeNull();
    });

    it('should return null on missing configuration', async () => {
      const mockResponse = createMockSOAPResponse('<GetVideoEncoderConfigurationResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfiguration('invalid');

      expect(result).toBeNull();
    });
  });

  describe('setVideoEncoderConfiguration', () => {
    it('should set video encoder configuration', async () => {
      const mockResponse = createMockSOAPResponse('<SetVideoEncoderConfigurationResponse />');

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
      const mockResponse = createMockSOAPResponse('<SetVideoEncoderConfigurationResponse />');

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
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Set failed');

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
      const mockResponse = createMockSOAPResponse(`
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
      `);

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
      const mockResponse = createMockSOAPResponse(`
        <GetVideoEncoderConfigurationOptionsResponse>
          <Options>
            <QualityRange><Min>0</Min><Max>100</Max></QualityRange>
          </Options>
        </GetVideoEncoderConfigurationOptionsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await getVideoEncoderConfigurationOptions('encoder_1', 'profile_1');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('encoder_1');
      expect(callArgs[1]).toContain('profile_1');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Get options failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getVideoEncoderConfigurationOptions()).rejects.toThrow();
    });

    it('should throw on missing Options', async () => {
      const mockResponse = createMockSOAPResponse(
        '<GetVideoEncoderConfigurationOptionsResponse />',
      );

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getVideoEncoderConfigurationOptions()).rejects.toThrow('Invalid response');
    });
  });

  describe('getProfiles', () => {
    it('should parse multiple profiles correctly', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetProfilesResponse>
          <Profiles token="ProfileToken1">
            <Name>MainStream</Name>
            <VideoSourceConfiguration token="VideoSourceToken1">
              <Name>Video Source 1</Name>
            </VideoSourceConfiguration>
            <VideoEncoderConfiguration token="VideoEncoderToken1">
              <Name>H.264 Encoder</Name>
              <Encoding>H264</Encoding>
            </VideoEncoderConfiguration>
            <AudioSourceConfiguration token="AudioSourceToken1">
              <Name>Audio Source 1</Name>
            </AudioSourceConfiguration>
            <AudioEncoderConfiguration token="AudioEncoderToken1">
              <Name>Audio Encoder 1</Name>
            </AudioEncoderConfiguration>
            <PTZConfiguration token="PTZToken1">
              <Name>PTZ Config 1</Name>
            </PTZConfiguration>
            <MetadataConfiguration token="MetadataToken1">
              <Name>Metadata Config 1</Name>
            </MetadataConfiguration>
          </Profiles>
          <Profiles token="ProfileToken2">
            <Name>SubStream</Name>
            <fixed>true</fixed>
          </Profiles>
        </GetProfilesResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getProfiles();

      expect(result).toHaveLength(2);
      expect(result[0].token).toBe('ProfileToken1');
      expect(result[0].name).toBe('MainStream');
      expect(result[0].videoSourceConfiguration).toEqual({
        token: 'VideoSourceToken1',
        name: 'Video Source 1',
      });
      expect(result[0].videoEncoderConfiguration).toEqual({
        token: 'VideoEncoderToken1',
        name: 'H.264 Encoder',
        encoding: 'H264',
      });
      expect(result[0].audioSourceConfiguration).toEqual({
        token: 'AudioSourceToken1',
        name: 'Audio Source 1',
      });
      expect(result[0].audioEncoderConfiguration).toEqual({
        token: 'AudioEncoderToken1',
        name: 'Audio Encoder 1',
      });
      expect(result[0].ptzConfiguration).toEqual({
        token: 'PTZToken1',
        name: 'PTZ Config 1',
      });
      expect(result[0].metadataConfiguration).toEqual({
        token: 'MetadataToken1',
        name: 'Metadata Config 1',
      });
      expect(result[1].token).toBe('ProfileToken2');
      expect(result[1].name).toBe('SubStream');
      expect(result[1].fixed).toBe(true);
    });

    it('should parse single profile correctly', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetProfilesResponse>
          <Profiles token="ProfileToken1">
            <Name>MainStream</Name>
          </Profiles>
        </GetProfilesResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getProfiles();

      expect(result).toHaveLength(1);
      expect(result[0].token).toBe('ProfileToken1');
      expect(result[0].name).toBe('MainStream');
    });

    it('should return empty array when no profiles', async () => {
      const mockResponse = createMockSOAPResponse('<GetProfilesResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getProfiles();

      expect(result).toEqual([]);
    });

    it('should handle profile with fixed as string', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetProfilesResponse>
          <Profiles token="ProfileToken1">
            <Name>MainStream</Name>
            <fixed>true</fixed>
          </Profiles>
        </GetProfilesResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getProfiles();

      expect(result[0].fixed).toBe(true);
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Operation failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getProfiles()).rejects.toThrow();
    });
  });

  describe('getProfile', () => {
    it('should parse single profile by token', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetProfileResponse>
          <Profile token="ProfileToken1">
            <Name>MainStream</Name>
          </Profile>
        </GetProfileResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getProfile('ProfileToken1');

      expect(result).not.toBeNull();
      expect(result?.token).toBe('ProfileToken1');
      expect(result?.name).toBe('MainStream');
    });

    it('should return null on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Not found');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getProfile('invalid');

      expect(result).toBeNull();
    });

    it('should return null on missing profile', async () => {
      const mockResponse = createMockSOAPResponse('<GetProfileResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getProfile('invalid');

      expect(result).toBeNull();
    });
  });

  describe('createProfile', () => {
    it('should create profile and return token', async () => {
      const mockResponse = createMockSOAPResponse(`
        <CreateProfileResponse>
          <Profile token="NewProfileToken" />
        </CreateProfileResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await createProfile('New Profile');

      expect(result).toBe('NewProfileToken');
      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('New Profile');
    });

    it('should escape XML in profile name', async () => {
      const mockResponse = createMockSOAPResponse(`
        <CreateProfileResponse>
          <Profile token="NewProfileToken" />
        </CreateProfileResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await createProfile('Profile <with> XML');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('&lt;');
      expect(callArgs[1]).toContain('&gt;');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Create failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(createProfile('New Profile')).rejects.toThrow();
    });
  });

  describe('deleteProfile', () => {
    it('should delete profile successfully', async () => {
      const mockResponse = createMockSOAPResponse('<DeleteProfileResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await deleteProfile('ProfileToken1');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('ProfileToken1');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Delete failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(deleteProfile('ProfileToken1')).rejects.toThrow();
    });
  });

  describe('getVideoEncoderConfigurationOptions - helper functions', () => {
    it('should parse H264 options with single resolution', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetVideoEncoderConfigurationOptionsResponse>
          <Options>
            <QualityRange><Min>0</Min><Max>100</Max></QualityRange>
            <H264>
              <ResolutionsAvailable>
                <Width>1920</Width>
                <Height>1080</Height>
              </ResolutionsAvailable>
              <FrameRateRange><Min>1</Min><Max>30</Max></FrameRateRange>
              <EncodingIntervalRange><Min>1</Min><Max>30</Max></EncodingIntervalRange>
              <BitrateRange><Min>64</Min><Max>8192</Max></BitrateRange>
              <H264ProfilesSupported>Main</H264ProfilesSupported>
              <GovLengthRange><Min>1</Min><Max>300</Max></GovLengthRange>
            </H264>
          </Options>
        </GetVideoEncoderConfigurationOptionsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfigurationOptions();

      expect(result.h264?.resolutionsAvailable).toHaveLength(1);
      expect(result.h264?.resolutionsAvailable[0]).toEqual({ width: 1920, height: 1080 });
    });

    it('should parse JPEG options correctly', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetVideoEncoderConfigurationOptionsResponse>
          <Options>
            <QualityRange><Min>0</Min><Max>100</Max></QualityRange>
            <JPEG>
              <ResolutionsAvailable>
                <Width>1920</Width>
                <Height>1080</Height>
              </ResolutionsAvailable>
              <ResolutionsAvailable>
                <Width>1280</Width>
                <Height>720</Height>
              </ResolutionsAvailable>
              <FrameRateRange><Min>1</Min><Max>30</Max></FrameRateRange>
              <EncodingIntervalRange><Min>1</Min><Max>30</Max></EncodingIntervalRange>
            </JPEG>
          </Options>
        </GetVideoEncoderConfigurationOptionsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfigurationOptions();

      expect(result.jpeg?.resolutionsAvailable).toHaveLength(2);
      expect(result.jpeg?.resolutionsAvailable[0]).toEqual({ width: 1920, height: 1080 });
      expect(result.jpeg?.resolutionsAvailable[1]).toEqual({ width: 1280, height: 720 });
    });

    it('should handle missing H264 options', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetVideoEncoderConfigurationOptionsResponse>
          <Options>
            <QualityRange><Min>0</Min><Max>100</Max></QualityRange>
          </Options>
        </GetVideoEncoderConfigurationOptionsResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getVideoEncoderConfigurationOptions();

      expect(result.h264).toBeUndefined();
      expect(result.jpeg).toBeUndefined();
    });
  });
});
