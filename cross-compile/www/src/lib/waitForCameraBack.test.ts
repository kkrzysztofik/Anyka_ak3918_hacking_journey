import { describe, expect, it, vi } from 'vitest';

import { waitForCameraBack } from '@/lib/waitForCameraBack';

describe('waitForCameraBack', () => {
  it('resolves saw-down true once the probe fails and then succeeds again', async () => {
    const probe = vi
      .fn()
      .mockResolvedValueOnce(undefined) // still up (pre-reboot)
      .mockRejectedValueOnce(new Error('down'))
      .mockResolvedValueOnce(undefined); // back
    const result = await waitForCameraBack(probe, { intervalMs: 0, timeoutMs: 5000 });
    expect(result).toBe('back');
  });

  it('reports a timeout without ever seeing the camera go down', async () => {
    const probe = vi.fn().mockResolvedValue(undefined);
    const result = await waitForCameraBack(probe, { intervalMs: 0, timeoutMs: 0 });
    expect(result).toBe('never-went-down');
  });

  it('reports still-unreachable when it went down and never came back', async () => {
    const probe = vi.fn().mockRejectedValue(new Error('down'));
    const result = await waitForCameraBack(probe, { intervalMs: 0, timeoutMs: 10 });
    expect(result).toBe('still-down');
  });
});
