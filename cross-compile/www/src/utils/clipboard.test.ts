/**
 * Clipboard helper tests — Clipboard API plus HTTP fallback.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { copyTextToClipboard } from './clipboard';

function mockExecCommand(result: boolean): ReturnType<typeof vi.fn> {
  const exec = vi.fn().mockReturnValue(result);
  Object.defineProperty(document, 'execCommand', {
    value: exec,
    configurable: true,
    writable: true,
  });
  return exec;
}

describe('copyTextToClipboard', () => {
  const originalClipboard = navigator.clipboard;

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    Object.defineProperty(navigator, 'clipboard', {
      value: originalClipboard,
      configurable: true,
      writable: true,
    });
  });

  it('uses navigator.clipboard.writeText when available', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText },
      configurable: true,
      writable: true,
    });

    await expect(copyTextToClipboard('http://cam/live/main.flv')).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith('http://cam/live/main.flv');
  });

  it('falls back to execCommand when clipboard is missing (plain HTTP)', async () => {
    Object.defineProperty(navigator, 'clipboard', {
      value: undefined,
      configurable: true,
      writable: true,
    });
    const exec = mockExecCommand(true);

    await expect(copyTextToClipboard('http://192.168.2.198:8080/live/main.flv')).resolves.toBe(
      true,
    );
    expect(exec).toHaveBeenCalledWith('copy');
  });

  it('falls back to execCommand when writeText rejects', async () => {
    const writeText = vi.fn().mockRejectedValue(new Error('NotAllowedError'));
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText },
      configurable: true,
      writable: true,
    });
    const exec = mockExecCommand(true);

    await expect(copyTextToClipboard('fallback-text')).resolves.toBe(true);
    expect(exec).toHaveBeenCalledWith('copy');
  });

  it('returns false when both clipboard and execCommand fail', async () => {
    Object.defineProperty(navigator, 'clipboard', {
      value: undefined,
      configurable: true,
      writable: true,
    });
    mockExecCommand(false);

    await expect(copyTextToClipboard('nope')).resolves.toBe(false);
  });
});
