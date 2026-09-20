/**
 * Wait for the camera to disappear and come back.
 *
 * Shared by FirmwareUpgradeDialog (post-upload reboot) and ProcessesCard
 * (onvif restart): both need the down→up edge, and the version/message logic
 * stays in the caller, keyed off the returned outcome.
 */

export type WaitOutcome = 'back' | 'still-down' | 'never-went-down';

function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(new DOMException('Aborted', 'AbortError'));
      return;
    }
    const onAbort = () => {
      window.clearTimeout(id);
      reject(new DOMException('Aborted', 'AbortError'));
    };
    const id = window.setTimeout(() => {
      signal?.removeEventListener('abort', onAbort);
      resolve();
    }, ms);
    // `once` so the abort path cleans up too: this helper is called in a loop
    // against a single long-lived signal, and only the resolve path detaches.
    signal?.addEventListener('abort', onAbort, { once: true });
  });
}

export function isAbortError(err: unknown): boolean {
  return err instanceof DOMException
    ? err.name === 'AbortError'
    : err instanceof Error && err.name === 'AbortError';
}

/**
 * Poll `probe` until it fails once (camera down) and then succeeds again
 * (camera back), or the timeout elapses.
 *
 * ponytail: the down→up edge approximates a reconnect. Upgrade path if false
 * positives appear: a trial-status API that reports the reboot explicitly.
 */
export async function waitForCameraBack(
  probe: (signal?: AbortSignal) => Promise<unknown>,
  opts: { intervalMs: number; timeoutMs: number; signal?: AbortSignal },
): Promise<WaitOutcome> {
  const { intervalMs, timeoutMs, signal } = opts;
  const deadline = Date.now() + timeoutMs;
  let sawDown = false;
  while (Date.now() < deadline) {
    if (signal?.aborted) throw new DOMException('Aborted', 'AbortError');
    try {
      await probe(signal);
      if (sawDown) return 'back';
      // Still reachable — keep polling.
    } catch (err) {
      if (isAbortError(err)) throw err;
      sawDown = true;
    }
    await sleep(intervalMs, signal);
  }
  return sawDown ? 'still-down' : 'never-went-down';
}
