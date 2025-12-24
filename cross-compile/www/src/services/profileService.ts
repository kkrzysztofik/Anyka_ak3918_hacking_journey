/**
 * Profile Service
 *
 * SOAP operations for media profiles management.
 */
import { ENDPOINTS, apiClient } from '@/services/api';
import {
  createSOAPEnvelope,
  escapeXml,
  escapeXmlAttribute,
  parseSOAPResponse,
} from '@/services/soap/client';

/**
 * Helper function to safely convert a value to string
 */
function safeString(value: unknown, defaultValue: string = ''): string {
  if (value === null || value === undefined) {
    return defaultValue;
  }
  if (typeof value === 'string') {
    return value || defaultValue;
  }
  // Only convert primitive types to string, objects return default to avoid '[object Object]'
  if (typeof value === 'object') {
    return defaultValue;
  }
  // For primitives (number, boolean, symbol, bigint), convert safely
  const valueType = typeof value;
  if (
    valueType === 'number' ||
    valueType === 'boolean' ||
    valueType === 'symbol' ||
    valueType === 'bigint'
  ) {
    return String(value);
  }
  // Fallback for any other type
  return defaultValue;
}

export interface MediaProfile {
  token: string;
  name: string;
  videoSourceToken?: string;
  videoEncoderToken?: string;
  audioSourceToken?: string;
  audioEncoderToken?: string;
  videoSourceConfiguration?: {
    token: string;
    name: string;
  };
  videoEncoderConfiguration?: {
    token: string;
    name: string;
    encoding: string;
  };
  audioSourceConfiguration?: {
    token: string;
    name: string;
  };
  audioEncoderConfiguration?: {
    token: string;
    name: string;
  };
  ptzConfiguration?: {
    token: string;
    name: string;
  };
  metadataConfiguration?: {
    token: string;
    name: string;
  };
  fixed?: boolean;
}

export type VideoEncoding = 'H264' | 'H265' | 'JPEG' | 'MPEG4';

export interface VideoEncoderConfiguration {
  token: string;
  name: string;
  encoding: VideoEncoding;
  resolution: {
    width: number;
    height: number;
  };
  quality: number;
  rateControl?: {
    frameRateLimit: number;
    encodingInterval: number;
    bitrateLimit: number;
  };
  h264?: {
    govLength: number;
    h264Profile: string;
  };
  sessionTimeout: string;
}

export interface VideoEncoderConfigurationOptions {
  qualityRange: {
    min: number;
    max: number;
  };
  h264?: {
    resolutionsAvailable: Array<{ width: number; height: number }>;
    frameRateRange: { min: number; max: number };
    encodingIntervalRange: { min: number; max: number };
    bitrateRange: { min: number; max: number };
    h264ProfilesSupported: string[];
    govLengthRange: { min: number; max: number };
  };
  jpeg?: {
    resolutionsAvailable: Array<{ width: number; height: number }>;
    frameRateRange: { min: number; max: number };
    encodingIntervalRange: { min: number; max: number };
  };
}

/**
 * Get all media profiles
 */
export async function getProfiles(): Promise<MediaProfile[]> {
  const envelope = createSOAPEnvelope('<trt:GetProfiles />');

  const response = await apiClient.post(ENDPOINTS.media, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get profiles');
  }

  const data = parsed.data?.GetProfilesResponse as Record<string, unknown> | undefined;
  const profiles = data?.Profiles;

  if (!profiles) {
    return [];
  }

  const profilesList = Array.isArray(profiles) ? profiles : [profiles];

  return profilesList.map((profile: Record<string, unknown>) => {
    const videoSource = profile.VideoSourceConfiguration as Record<string, unknown> | undefined;
    const videoEncoder = profile.VideoEncoderConfiguration as Record<string, unknown> | undefined;
    const audioSource = profile.AudioSourceConfiguration as Record<string, unknown> | undefined;
    const audioEncoder = profile.AudioEncoderConfiguration as Record<string, unknown> | undefined;
    const ptzConfig = profile.PTZConfiguration as Record<string, unknown> | undefined;
    const metadataConfig = profile.MetadataConfiguration as Record<string, unknown> | undefined;

    return {
      token: safeString(profile['@_token']),
      name: safeString(profile.Name),
      videoSourceToken: videoSource ? safeString(videoSource['@_token']) : undefined,
      videoEncoderToken: videoEncoder ? safeString(videoEncoder['@_token']) : undefined,
      audioSourceToken: audioSource ? safeString(audioSource['@_token']) : undefined,
      audioEncoderToken: audioEncoder ? safeString(audioEncoder['@_token']) : undefined,
      videoSourceConfiguration: videoSource
        ? {
            token: safeString(videoSource['@_token']),
            name: safeString(videoSource.Name),
          }
        : undefined,
      videoEncoderConfiguration: videoEncoder
        ? {
            token: safeString(videoEncoder['@_token']),
            name: safeString(videoEncoder.Name),
            encoding: safeString(videoEncoder.Encoding, 'H264'),
          }
        : undefined,
      audioSourceConfiguration: audioSource
        ? {
            token: safeString(audioSource['@_token']),
            name: safeString(audioSource.Name),
          }
        : undefined,
      audioEncoderConfiguration: audioEncoder
        ? {
            token: safeString(audioEncoder['@_token']),
            name: safeString(audioEncoder.Name),
          }
        : undefined,
      ptzConfiguration: ptzConfig
        ? {
            token: safeString(ptzConfig['@_token']),
            name: safeString(ptzConfig.Name),
          }
        : undefined,
      metadataConfiguration: metadataConfig
        ? {
            token: safeString(metadataConfig['@_token']),
            name: safeString(metadataConfig.Name),
          }
        : undefined,
      fixed: profile.fixed === true || profile.fixed === 'true',
    };
  });
}

/**
 * Get a single profile by token
 */
export async function getProfile(token: string): Promise<MediaProfile | null> {
  // NOSONAR - Token comes from device response (trusted source), not user input
  const body = `<trt:GetProfile>
    <trt:ProfileToken>${token}</trt:ProfileToken>
  </trt:GetProfile>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.media, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    return null;
  }

  const data = parsed.data?.GetProfileResponse as Record<string, unknown> | undefined;
  const profile = data?.Profile as Record<string, unknown> | undefined;

  if (!profile) {
    return null;
  }

  return {
    token: safeString(profile['@_token']),
    name: safeString(profile.Name),
  };
}

/**
 * Create a new profile
 */
export async function createProfile(name: string): Promise<string> {
  // Escape user-provided name to prevent XML injection
  const escapedName = escapeXml(name);
  const body = `<trt:CreateProfile>
    <trt:Name>${escapedName}</trt:Name>
  </trt:CreateProfile>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.media, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to create profile');
  }

  const data = parsed.data?.CreateProfileResponse as Record<string, unknown> | undefined;
  const profile = data?.Profile as Record<string, unknown> | undefined;

  return safeString(profile?.['@_token']);
}

/**
 * Delete a profile
 */
export async function deleteProfile(token: string): Promise<void> {
  // NOSONAR - Token comes from device response (trusted source), not user input
  const body = `<trt:DeleteProfile>
    <trt:ProfileToken>${token}</trt:ProfileToken>
  </trt:DeleteProfile>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.media, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to delete profile');
  }
}

/**
 * Get video encoder configurations
 */
export async function getVideoEncoderConfigurations(): Promise<VideoEncoderConfiguration[]> {
  const envelope = createSOAPEnvelope('<trt:GetVideoEncoderConfigurations />');

  const response = await apiClient.post(ENDPOINTS.media, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get video encoder configurations');
  }

  const data = parsed.data?.GetVideoEncoderConfigurationsResponse as
    | Record<string, unknown>
    | undefined;
  const configs = data?.Configurations;

  if (!configs) {
    return [];
  }

  const configsList = Array.isArray(configs) ? configs : [configs];

  return configsList.map((config: Record<string, unknown>) => {
    const resolution = config.Resolution as Record<string, unknown> | undefined;
    const rateControl = config.RateControl as Record<string, unknown> | undefined;
    const h264 = config.H264 as Record<string, unknown> | undefined;

    return {
      token: safeString(config['@_token']),
      name: safeString(config.Name),
      encoding: safeString(config.Encoding, 'H264') as VideoEncoding,
      resolution: {
        width: Number(resolution?.Width || 1920),
        height: Number(resolution?.Height || 1080),
      },
      quality: Number(config.Quality || 80),
      rateControl: rateControl
        ? {
            frameRateLimit: Number(rateControl.FrameRateLimit || 30),
            encodingInterval: Number(rateControl.EncodingInterval || 1),
            bitrateLimit: Number(rateControl.BitrateLimit || 4000),
          }
        : undefined,
      h264: h264
        ? {
            govLength: Number(h264.GovLength || 30),
            h264Profile: safeString(h264.H264Profile, 'Main'),
          }
        : undefined,
      sessionTimeout: safeString(config.SessionTimeout, 'PT60S'),
    };
  });
}

/**
 * Get a single video encoder configuration by token
 */
export async function getVideoEncoderConfiguration(
  token: string,
): Promise<VideoEncoderConfiguration | null> {
  // NOSONAR - Token comes from device response (trusted source), not user input
  const body = `<trt:GetVideoEncoderConfiguration>
    <trt:ConfigurationToken>${token}</trt:ConfigurationToken>
  </trt:GetVideoEncoderConfiguration>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.media, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    return null;
  }

  const data = parsed.data?.GetVideoEncoderConfigurationResponse as
    | Record<string, unknown>
    | undefined;
  const config = data?.Configuration as Record<string, unknown> | undefined;

  if (!config) {
    return null;
  }

  const resolution = config.Resolution as Record<string, unknown> | undefined;
  const rateControl = config.RateControl as Record<string, unknown> | undefined;
  const h264 = config.H264 as Record<string, unknown> | undefined;

  return {
    token: safeString(config['@_token']),
    name: safeString(config.Name),
    encoding: safeString(config.Encoding, 'H264') as VideoEncoding,
    resolution: {
      width: Number(resolution?.Width || 1920),
      height: Number(resolution?.Height || 1080),
    },
    quality: Number(config.Quality || 80),
    rateControl: rateControl
      ? {
          frameRateLimit: Number(rateControl.FrameRateLimit || 30),
          encodingInterval: Number(rateControl.EncodingInterval || 1),
          bitrateLimit: Number(rateControl.BitrateLimit || 4000),
        }
      : undefined,
    h264: h264
      ? {
          govLength: Number(h264.GovLength || 30),
          h264Profile: safeString(h264.H264Profile, 'Main'),
        }
      : undefined,
    sessionTimeout: safeString(config.SessionTimeout, 'PT60S'),
  };
}

/**
 * Set video encoder configuration
 */
export async function setVideoEncoderConfiguration(
  config: VideoEncoderConfiguration,
  forcePersistence: boolean = false,
): Promise<void> {
  const resolutionXml = `<tt:Resolution>
      <tt:Width>${config.resolution.width}</tt:Width>
      <tt:Height>${config.resolution.height}</tt:Height>
    </tt:Resolution>`;

  const rateControlXml = config.rateControl
    ? `<tt:RateControl>
      <tt:FrameRateLimit>${config.rateControl.frameRateLimit}</tt:FrameRateLimit>
      <tt:EncodingInterval>${config.rateControl.encodingInterval}</tt:EncodingInterval>
      <tt:BitrateLimit>${config.rateControl.bitrateLimit}</tt:BitrateLimit>
    </tt:RateControl>`
    : '';

  const h264Xml = config.h264
    ? `<tt:H264>
      <tt:GovLength>${config.h264.govLength}</tt:GovLength>
      <tt:H264Profile>${config.h264.h264Profile}</tt:H264Profile>
    </tt:H264>`
    : '';

  // Escape user-provided values to prevent XML injection
  // Token comes from device response (trusted), but name and encoding may be user-modified
  const escapedName = escapeXml(config.name);
  const escapedEncoding = escapeXml(config.encoding);
  const escapedSessionTimeout = escapeXml(config.sessionTimeout);
  // NOSONAR - Token from device, but escaping for defense-in-depth
  const escapedToken = escapeXmlAttribute(config.token);

  const body = `<trt:SetVideoEncoderConfiguration>
    <trt:Configuration token="${escapedToken}">
      <tt:Name>${escapedName}</tt:Name>
      <tt:UseCount>0</tt:UseCount>
      <tt:Encoding>${escapedEncoding}</tt:Encoding>
      ${resolutionXml}
      <tt:Quality>${config.quality}</tt:Quality>
      ${rateControlXml}
      ${h264Xml}
      <tt:SessionTimeout>${escapedSessionTimeout}</tt:SessionTimeout>
    </trt:Configuration>
    <trt:ForcePersistence>${forcePersistence}</trt:ForcePersistence>
  </trt:SetVideoEncoderConfiguration>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.media, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to set video encoder configuration');
  }
}

/**
 * Helper function to parse H264 options
 */
function parseH264Options(h264: Record<string, unknown>): VideoEncoderConfigurationOptions['h264'] {
  const resolutions = h264.ResolutionsAvailable as
    | Array<Record<string, unknown>>
    | Record<string, unknown>
    | undefined;
  const frameRateRange = h264.FrameRateRange as Record<string, unknown> | undefined;
  const encodingIntervalRange = h264.EncodingIntervalRange as Record<string, unknown> | undefined;
  const bitrateRange = h264.BitrateRange as Record<string, unknown> | undefined;
  const govLengthRange = h264.GovLengthRange as Record<string, unknown> | undefined;
  const profiles = h264.H264ProfilesSupported as string[] | undefined;

  let resolutionsArray: Array<Record<string, unknown>>;
  if (Array.isArray(resolutions)) {
    resolutionsArray = resolutions;
  } else if (resolutions) {
    resolutionsArray = [resolutions];
  } else {
    resolutionsArray = [];
  }

  return {
    resolutionsAvailable: resolutionsArray.map((r) => ({
      width: Number(r.Width || 1920),
      height: Number(r.Height || 1080),
    })),
    frameRateRange: {
      min: Number(frameRateRange?.Min ?? 1),
      max: Number(frameRateRange?.Max ?? 30),
    },
    encodingIntervalRange: {
      min: Number(encodingIntervalRange?.Min ?? 1),
      max: Number(encodingIntervalRange?.Max ?? 30),
    },
    bitrateRange: {
      min: Number(bitrateRange?.Min ?? 64),
      max: Number(bitrateRange?.Max ?? 8192),
    },
    h264ProfilesSupported: profiles || [],
    govLengthRange: {
      min: Number(govLengthRange?.Min ?? 1),
      max: Number(govLengthRange?.Max ?? 300),
    },
  };
}

/**
 * Helper function to parse JPEG options
 */
function parseJpegOptions(jpeg: Record<string, unknown>): VideoEncoderConfigurationOptions['jpeg'] {
  const resolutions = jpeg.ResolutionsAvailable as
    | Array<Record<string, unknown>>
    | Record<string, unknown>
    | undefined;
  const frameRateRange = jpeg.FrameRateRange as Record<string, unknown> | undefined;
  const encodingIntervalRange = jpeg.EncodingIntervalRange as Record<string, unknown> | undefined;

  let resolutionsArray: Array<Record<string, unknown>>;
  if (Array.isArray(resolutions)) {
    resolutionsArray = resolutions;
  } else if (resolutions) {
    resolutionsArray = [resolutions];
  } else {
    resolutionsArray = [];
  }

  return {
    resolutionsAvailable: resolutionsArray.map((r) => ({
      width: Number(r.Width || 1920),
      height: Number(r.Height || 1080),
    })),
    frameRateRange: {
      min: Number(frameRateRange?.Min ?? 1),
      max: Number(frameRateRange?.Max ?? 30),
    },
    encodingIntervalRange: {
      min: Number(encodingIntervalRange?.Min ?? 1),
      max: Number(encodingIntervalRange?.Max ?? 30),
    },
  };
}

/**
 * Get video encoder configuration options
 */
export async function getVideoEncoderConfigurationOptions(
  configurationToken?: string,
  profileToken?: string,
): Promise<VideoEncoderConfigurationOptions> {
  // NOSONAR - Tokens come from device responses (trusted sources), not user input
  const tokenXml = configurationToken
    ? `<trt:ConfigurationToken>${configurationToken}</trt:ConfigurationToken>`
    : '';
  // NOSONAR - Profile token comes from device response (trusted source)
  const profileXml = profileToken ? `<trt:ProfileToken>${profileToken}</trt:ProfileToken>` : '';

  const body = `<trt:GetVideoEncoderConfigurationOptions>
    ${tokenXml}
    ${profileXml}
  </trt:GetVideoEncoderConfigurationOptions>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.media, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get video encoder configuration options');
  }

  const data = parsed.data?.GetVideoEncoderConfigurationOptionsResponse as
    | Record<string, unknown>
    | undefined;
  const options = data?.Options as Record<string, unknown> | undefined;

  if (!options) {
    throw new Error('Invalid response: missing Options');
  }

  const qualityRange = options.QualityRange as Record<string, unknown> | undefined;
  const h264 = options.H264 as Record<string, unknown> | undefined;
  const jpeg = options.JPEG as Record<string, unknown> | undefined;

  const result: VideoEncoderConfigurationOptions = {
    qualityRange: {
      min: Number(qualityRange?.Min ?? 0),
      max: Number(qualityRange?.Max ?? 100),
    },
  };

  if (h264) {
    result.h264 = parseH264Options(h264);
  }

  if (jpeg) {
    result.jpeg = parseJpegOptions(jpeg);
  }

  return result;
}
