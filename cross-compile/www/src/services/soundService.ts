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

  if (!response.ok) {
    // 409 carries {"status":"busy"}, which is what the /busy/i match keys on.
    const text = await response.text();
    throw new Error(text || `Play sound failed (${response.status})`);
  }

  const { status } = (await response.json()) as { status?: string };
  if (status === 'accepted' || status === 'debounced') {
    return status;
  }

  throw new Error(`Unexpected play sound status: ${String(status)}`);
}
