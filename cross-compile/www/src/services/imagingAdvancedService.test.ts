/**
 * imagingAdvancedService Tests
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { authorizedFetch } from './api';
import { getAdvancedImaging, putAdvancedImaging } from './imagingAdvancedService';

vi.mock('./api', () => ({
  authorizedFetch: vi.fn(),
}));

describe('imagingAdvancedService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('maps the snake_case GET response to camelCase', async () => {
    vi.mocked(authorizedFetch).mockResolvedValue(
      Response.json({ hue: 60, power_hz: 60, style_id: 1 }),
    );

    await expect(getAdvancedImaging()).resolves.toEqual({
      hue: 60,
      powerHz: 60,
      styleId: 1,
    });
  });

  it('PUT sends snake_case keys for the fields that were set', async () => {
    vi.mocked(authorizedFetch).mockResolvedValue(Response.json({}));

    await putAdvancedImaging({ powerHz: 60 });

    expect(authorizedFetch).toHaveBeenCalledWith(
      '/api/imaging',
      expect.objectContaining({
        method: 'PUT',
        body: JSON.stringify({ power_hz: 60 }),
      }),
    );
  });

  it('surfaces the server reason from a 400 body, not the raw JSON', async () => {
    vi.mocked(authorizedFetch).mockResolvedValue(
      new Response(JSON.stringify({ error: 'power_hz must be 50 or 60 (got 55)' }), {
        status: 400,
      }),
    );

    await expect(putAdvancedImaging({ powerHz: 55 })).rejects.toThrow(
      'power_hz must be 50 or 60 (got 55)',
    );
  });
});
