/**
 * Sound Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { authorizedFetch } from '@/services/api';
import { getSoundStatus, playSound } from '@/services/soundService';

vi.mock('@/services/api', () => ({
  authorizedFetch: vi.fn(),
}));

describe('soundService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getSoundStatus', () => {
    it('should parse enabled and events from GET /api/sound', async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            enabled: true,
            events: [
              { id: 'boot_ready', clip: 'boot.wav' },
              { id: 'motion', clip: 'beep.wav' },
            ],
          }),
          { status: 200 },
        ),
      );

      const status = await getSoundStatus();

      expect(authorizedFetch).toHaveBeenCalledWith(
        '/api/sound',
        expect.objectContaining({ method: 'GET' }),
      );
      expect(status.enabled).toBe(true);
      expect(status.events).toEqual([
        { id: 'boot_ready', clip: 'boot.wav' },
        { id: 'motion', clip: 'beep.wav' },
      ]);
    });
  });

  describe('playSound', () => {
    it("should POST event and resolve 'accepted' on 200", async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(
        new Response(JSON.stringify({ status: 'accepted' }), { status: 200 }),
      );

      await expect(playSound('boot_ready')).resolves.toBe('accepted');

      expect(authorizedFetch).toHaveBeenCalledWith(
        '/api/sound/play',
        expect.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ event: 'boot_ready' }),
        }),
      );
    });

    it("should resolve 'debounced' on 200 with status debounced", async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(
        new Response(JSON.stringify({ status: 'debounced' }), { status: 200 }),
      );

      await expect(playSound('boot_ready')).resolves.toBe('debounced');
    });

    it('should reject with busy message on 409', async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(
        new Response(JSON.stringify({ status: 'busy' }), { status: 409 }),
      );

      await expect(playSound('boot_ready')).rejects.toThrow(/busy/i);
    });

    it('should reject on 404', async () => {
      vi.mocked(authorizedFetch).mockResolvedValueOnce(
        new Response('sound unavailable or unknown event', { status: 404 }),
      );

      await expect(playSound('unknown_event')).rejects.toThrow();
    });
  });
});
