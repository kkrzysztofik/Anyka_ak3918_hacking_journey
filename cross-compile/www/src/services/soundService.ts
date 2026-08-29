/**
 * Sound Service
 *
 * REST client for GET /api/sound and POST /api/sound/play.
 */
import { authorizedFetch } from '@/services/api';

export interface SoundEventItem {
  id: string;
  clip: string;
}

export interface SoundStatus {
  enabled: boolean;
  events: SoundEventItem[];
}

export type PlaySoundStatus = 'accepted' | 'debounced';

/**
 * Read configured sound events and enabled flag from GET /api/sound.
 */
export async function getSoundStatus(signal?: AbortSignal): Promise<SoundStatus> {
  const response = await authorizedFetch('/api/sound', { method: 'GET', signal });
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `Failed to load sound status (${response.status})`);
  }
  return (await response.json()) as SoundStatus;
}

/**
 * Trigger playback of a configured event clip via POST /api/sound/play.
 *
 * HTTP 409 (player busy) is thrown as an Error whose message matches /busy/i
 * so React Query mutations can surface it via onError.
 */
export async function playSound(event: string, signal?: AbortSignal): Promise<PlaySoundStatus> {
  const response = await authorizedFetch('/api/sound/play', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ event }),
    signal,
  });

  if (response.status === 409) {
    const text = await response.text();
    throw new Error(text || 'busy');
  }

  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `Play sound failed (${response.status})`);
  }

  let payload: unknown;
  try {
    payload = await response.json();
  } catch {
    throw new Error('Invalid JSON in play sound response');
  }

  const status =
    typeof payload === 'object' &&
    payload !== null &&
    'status' in payload &&
    typeof (payload as { status: unknown }).status === 'string'
      ? (payload as { status: string }).status
      : undefined;

  if (status === 'accepted' || status === 'debounced') {
    return status;
  }

  throw new Error(`Unexpected play sound status: ${String(status)}`);
}
