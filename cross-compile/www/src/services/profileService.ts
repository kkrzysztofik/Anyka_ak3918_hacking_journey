/**
 * Profile Service
 *
 * SOAP operations for media profiles management.
 */
import { ENDPOINTS, apiClient } from '@/services/api';
import { createSOAPEnvelope, parseSOAPResponse } from '@/services/soap/client';

export interface MediaProfile {
  token: string;
  name: string;
  videoSourceToken?: string;
  videoEncoderToken?: string;
  audioSourceToken?: string;
  audioEncoderToken?: string;
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

    return {
      token: String(profile['@_token'] || ''),
      name: String(profile.Name || ''),
      videoSourceToken: videoSource ? String(videoSource['@_token'] || '') : undefined,
      videoEncoderToken: videoEncoder ? String(videoEncoder['@_token'] || '') : undefined,
      audioSourceToken: audioSource ? String(audioSource['@_token'] || '') : undefined,
      audioEncoderToken: audioEncoder ? String(audioEncoder['@_token'] || '') : undefined,
    };
  });
}

/**
 * Get a single profile by token
 */
export async function getProfile(token: string): Promise<MediaProfile | null> {
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
    token: String(profile['@_token'] || ''),
    name: String(profile.Name || ''),
  };
}

/**
 * Create a new profile
 */
export async function createProfile(name: string): Promise<string> {
  const body = `<trt:CreateProfile>
    <trt:Name>${name}</trt:Name>
  </trt:CreateProfile>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.media, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to create profile');
  }

  const data = parsed.data?.CreateProfileResponse as Record<string, unknown> | undefined;
  const profile = data?.Profile as Record<string, unknown> | undefined;

  return String(profile?.['@_token'] || '');
}

/**
 * Delete a profile
 */
export async function deleteProfile(token: string): Promise<void> {
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
