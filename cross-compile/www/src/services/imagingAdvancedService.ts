/**
 * Advanced Imaging Service
 *
 * JSON operations for the ISP knobs ONVIF does not model (hue, mains
 * frequency, picture style), served by /api/imaging.
 */
import { authorizedFetch } from '@/services/api';

export interface AdvancedImaging {
  /** Colour tint, 0-100 (50 = neutral). */
  hue: number;
  /** Mains frequency for flicker reduction; 50 or 60. */
  powerHz: number;
  /** ISP picture-style id; 0-2. */
  styleId: number;
}

/**
 * Read the current advanced imaging values from GET /api/imaging.
 */
export async function getAdvancedImaging(): Promise<AdvancedImaging> {
  const response = await authorizedFetch('/api/imaging');
  if (!response.ok) {
    throw new Error(`Failed to load advanced imaging (${response.status})`);
  }
  const j = (await response.json()) as {
    hue: number;
    power_hz: number;
    style_id: number;
  };
  return { hue: j.hue, powerHz: j.power_hz, styleId: j.style_id };
}

/**
 * Write advanced imaging values via PUT /api/imaging (partial patch).
 */
export async function putAdvancedImaging(patch: Partial<AdvancedImaging>): Promise<void> {
  const body: Record<string, number> = {};
  if (patch.hue !== undefined) body.hue = patch.hue;
  if (patch.powerHz !== undefined) body.power_hz = patch.powerHz;
  if (patch.styleId !== undefined) body.style_id = patch.styleId;

  const response = await authorizedFetch('/api/imaging', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const text = await response.text();
    // The server answers {"error":"..."} on a 400 — surface the reason, not
    // the raw JSON, in the toast.
    let message = text;
    try {
      const parsed = JSON.parse(text) as { error?: string };
      if (parsed.error) message = parsed.error;
    } catch {
      // not JSON; keep the raw text
    }
    throw new Error(message || `Advanced imaging save failed (${response.status})`);
  }
}
