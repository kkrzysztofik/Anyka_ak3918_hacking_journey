# Live Video Preview Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `LiveViewPage` show actual moving video from the camera, and feed
its currently-hardcoded stats cards from the real stream.

**Architecture:** The camera already serves HTTP-FLV on port 8080 with
permissive CORS. No browser plays FLV natively, so `mpegts.js` demuxes it in
JavaScript and remuxes to fMP4 for Media Source Extensions, which drives a plain
`<video>` element. Zero server-side changes; everything in this plan is under
`cross-compile/www`.

**Tech Stack:** React 19, TypeScript, Vite (Rolldown), Vitest + React Testing
Library, `mpegts.js@1.8.1`, Tailwind, shadcn/ui.

**Design doc:** `docs/plans/2026-08-13-live-video-preview-design.md`

**Relevant skills:** @camera-webui-components, @anyka-webui-testing

---

## Background the implementer needs

You are working in `cross-compile/www`, a React SPA that is built by Vite
straight into `SD_card_contents/anyka_hack/onvif/www` and served off the
camera's SD card by a Rust binary. Some facts that are not obvious:

- **The page is served from port 80; the video stream lives on port 8080.**
  That is why URLs cannot simply be relative in production.
- **CORS is already handled** by the camera
  (`streaming-lib/src/protocol/httpflv/server.rs:87` sends
  `Access-Control-Allow-Origin: *`). Do not add a proxy for production.
- **Auth is Basic, and the password is AES-GCM encrypted in sessionStorage.**
  Never read `sessionStorage` directly. The only correct accessor is
  `useAuth().getBasicAuthHeader()`, which is **async** and returns
  `"Basic <base64>"` or `null`.
- **`renderWithProviders` already wraps `AuthProvider` and `QueryClientProvider`**
  (`src/test/componentTestHelpers.tsx:45`). Use it, not RTL's bare `render`.
- **The dev server proxies to a real camera** at `192.168.2.198:80`
  (`vite.config.ts:80`). That camera is the one healthy device in the fleet —
  develop against it.
- Components in `src/components/common/` use `readonly` props, a JSDoc header,
  `data-testid` on anything a test touches, and `cn()` from `@/lib/utils`.
  Read `src/components/common/ConnectionStatus.tsx` for the house style.

**Commands** (run from `cross-compile/www`):

| Purpose | Command |
| --- | --- |
| One test file | `npx vitest run src/path/to/file.test.ts` |
| All tests | `npm run test` |
| Lint | `npm run lint` |
| Types | `npm run type-check` |
| Production build | `npm run build` |
| Dev server | `npm run dev` |

---

## Task 1: Add the dependency and the dev proxy

**Files:**
- Modify: `cross-compile/www/package.json`
- Modify: `cross-compile/www/vite.config.ts:76-97`

**Step 1: Install mpegts.js**

```bash
cd cross-compile/www
npm install mpegts.js@^1.8.1
```

**Step 2: Add a `/live` dev proxy**

In `vite.config.ts`, inside the existing `server.proxy` object, after the
`/snapshot` entry, add a fourth entry. Note it targets **port 8080**, unlike the
other three which target port 80:

```ts
      // Proxy HTTP-FLV live streams. The FLV server listens on 8080 while the
      // WebUI is served from 80, so this entry rewrites the port for dev only.
      // NOSONAR: S5332, S4830 - HTTP and secure:false are required for embedded camera devices
      '/live': {
        target: (process.env.VITE_API_TARGET || 'http://192.168.2.198:80') // NOSONAR
          .replace(/:\d+$/, ':8080'),
        changeOrigin: true,
        secure: false, // NOSONAR
      },
```

**Step 3: Verify the config still parses**

Run: `npm run type-check`
Expected: PASS, no errors.

**Step 4: Commit**

```bash
git add cross-compile/www/package.json cross-compile/www/package-lock.json cross-compile/www/vite.config.ts
git commit -m "build(www): add mpegts.js and a dev proxy for HTTP-FLV"
```

---

## Task 2: Stream URL builder

A pure function, so it is tested exhaustively and cheaply. It exists because the
prod/dev port split is the single easiest thing to get wrong in this feature.

**Files:**
- Create: `cross-compile/www/src/utils/streamUrl.ts`
- Test: `cross-compile/www/src/utils/streamUrl.test.ts`

**Step 1: Write the failing test**

```ts
/**
 * Stream URL builder tests
 */
import { describe, expect, it } from 'vitest';

import { HTTPFLV_PORT, buildFlvUrl } from './streamUrl';

describe('buildFlvUrl', () => {
  it('builds an absolute URL on the FLV port in production', () => {
    expect(buildFlvUrl('main', { isDev: false, hostname: '192.168.2.198' })).toBe(
      'http://192.168.2.198:8080/live/main.flv',
    );
  });

  it('uses the sub stream path when asked', () => {
    expect(buildFlvUrl('sub', { isDev: false, hostname: '192.168.2.198' })).toBe(
      'http://192.168.2.198:8080/live/sub.flv',
    );
  });

  it('returns a relative path in development so the Vite proxy handles it', () => {
    expect(buildFlvUrl('main', { isDev: true, hostname: 'localhost' })).toBe('/live/main.flv');
  });

  it('exposes the FLV port so callers need not hardcode it', () => {
    expect(HTTPFLV_PORT).toBe(8080);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/utils/streamUrl.test.ts`
Expected: FAIL — cannot resolve `./streamUrl`.

**Step 3: Write minimal implementation**

```ts
/**
 * HTTP-FLV stream URL construction.
 *
 * The WebUI is served from port 80 but the camera's HTTP-FLV server listens on
 * 8080, so production URLs must be absolute and cross-port. The camera sends
 * `Access-Control-Allow-Origin: *`, which is what makes that legal from the
 * browser. In development the Vite `/live` proxy points at 8080 for us, so a
 * relative path keeps the request same-origin and avoids a CORS preflight.
 */
export type StreamType = 'main' | 'sub';

/** Port the camera's HTTP-FLV server listens on (config.toml: media.httpflv_port). */
export const HTTPFLV_PORT = 8080;

interface BuildFlvUrlOptions {
  readonly isDev?: boolean;
  readonly hostname?: string;
}

export function buildFlvUrl(streamType: StreamType, options: BuildFlvUrlOptions = {}): string {
  const { isDev = import.meta.env.DEV, hostname = window.location.hostname } = options;

  if (isDev) {
    return `/live/${streamType}.flv`;
  }
  return `http://${hostname}:${HTTPFLV_PORT}/live/${streamType}.flv`;
}
```

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/utils/streamUrl.test.ts`
Expected: PASS, 4 tests.

**Step 5: Commit**

```bash
git add cross-compile/www/src/utils/streamUrl.ts cross-compile/www/src/utils/streamUrl.test.ts
git commit -m "feat(www): add HTTP-FLV stream URL builder"
```

---

## Task 3: LiveVideoPlayer — lifecycle

The riskiest part of the feature is leaking a player on unmount or stream
switch, because each leaked player holds an open HTTP connection to a camera
with 36 MB of RAM. Test that first.

**Files:**
- Create: `cross-compile/www/src/components/common/LiveVideoPlayer.tsx`
- Test: `cross-compile/www/src/components/common/LiveVideoPlayer.test.tsx`

**Step 1: Write the failing test**

```tsx
/**
 * LiveVideoPlayer Tests
 */
import { waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '@/test/componentTestHelpers';

import { LiveVideoPlayer } from './LiveVideoPlayer';

const mockPlayer = {
  on: vi.fn(),
  attachMediaElement: vi.fn(),
  load: vi.fn(),
  destroy: vi.fn(),
};
const createPlayer = vi.fn(() => mockPlayer);

vi.mock('mpegts.js', () => ({
  default: {
    isSupported: () => true,
    createPlayer: (...args: unknown[]) => createPlayer(...args),
    Events: {
      MEDIA_INFO: 'media_info',
      STATISTICS_INFO: 'statistics_info',
      ERROR: 'error',
    },
  },
}));

vi.mock('@/hooks/useAuth', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/hooks/useAuth')>()),
  useAuth: () => ({ getBasicAuthHeader: async () => 'Basic dXNlcjpwYXNz' }),
}));

describe('LiveVideoPlayer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('creates a player pointed at the requested stream', async () => {
    renderWithProviders(<LiveVideoPlayer streamType="main" />);

    await waitFor(() => expect(createPlayer).toHaveBeenCalled());
    expect(createPlayer.mock.calls[0][0]).toMatchObject({
      type: 'flv',
      isLive: true,
      url: '/live/main.flv',
    });
  });

  it('passes the Basic auth header through to the player', async () => {
    renderWithProviders(<LiveVideoPlayer streamType="main" />);

    await waitFor(() => expect(createPlayer).toHaveBeenCalled());
    expect(createPlayer.mock.calls[0][1]).toMatchObject({
      headers: { Authorization: 'Basic dXNlcjpwYXNz' },
    });
  });

  it('leaves live latency chasing off', async () => {
    renderWithProviders(<LiveVideoPlayer streamType="main" />);

    await waitFor(() => expect(createPlayer).toHaveBeenCalled());
    expect(createPlayer.mock.calls[0][1]).toMatchObject({
      liveBufferLatencyChasing: false,
      enableStashBuffer: false,
    });
  });

  it('destroys the player on unmount so the connection is released', async () => {
    const { unmount } = renderWithProviders(<LiveVideoPlayer streamType="main" />);

    await waitFor(() => expect(createPlayer).toHaveBeenCalled());
    unmount();
    await waitFor(() => expect(mockPlayer.destroy).toHaveBeenCalledTimes(1));
  });

  it('destroys and recreates the player when the stream type changes', async () => {
    const { rerender } = renderWithProviders(<LiveVideoPlayer streamType="main" />);
    await waitFor(() => expect(createPlayer).toHaveBeenCalledTimes(1));

    rerender(<LiveVideoPlayer streamType="sub" />);

    await waitFor(() => expect(mockPlayer.destroy).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(createPlayer).toHaveBeenCalledTimes(2));
    expect(createPlayer.mock.calls[1][0]).toMatchObject({ url: '/live/sub.flv' });
  });

  it('does not recreate the player when only a callback identity changes', async () => {
    const { rerender } = renderWithProviders(
      <LiveVideoPlayer streamType="main" onStateChange={() => {}} />,
    );
    await waitFor(() => expect(createPlayer).toHaveBeenCalledTimes(1));

    rerender(<LiveVideoPlayer streamType="main" onStateChange={() => {}} />);

    expect(createPlayer).toHaveBeenCalledTimes(1);
    expect(mockPlayer.destroy).not.toHaveBeenCalled();
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/components/common/LiveVideoPlayer.test.tsx`
Expected: FAIL — cannot resolve `./LiveVideoPlayer`.

**Step 3: Write minimal implementation**

The last test is the important one. Parent components pass inline arrow
functions; if the effect depended on them, every parent render would tear down
and rebuild the video connection. Callbacks therefore go in a ref, and the
effect depends only on `streamType`.

```tsx
/**
 * Live Video Player
 *
 * Plays the camera's HTTP-FLV stream. No browser decodes FLV natively, so
 * mpegts.js demuxes it in JS and remuxes to fMP4 for Media Source Extensions.
 * The library is loaded with a dynamic import so it stays inside the lazy
 * LiveViewPage chunk rather than the eager bundle.
 */
import React, { useEffect, useRef } from 'react';

import { useAuth } from '@/hooks/useAuth';
import { cn } from '@/lib/utils';
import { type StreamType, buildFlvUrl } from '@/utils/streamUrl';

export type PlayerState = 'connecting' | 'playing' | 'stalled' | 'error';

export interface StreamStats {
  readonly width?: number;
  readonly height?: number;
  readonly fps?: number;
  readonly videoCodec?: string;
  readonly bitrateKbps?: number;
  readonly droppedFrames?: number;
}

interface LiveVideoPlayerProps {
  readonly streamType: StreamType;
  readonly onStateChange?: (state: PlayerState, message?: string) => void;
  readonly onStats?: (stats: StreamStats) => void;
  readonly className?: string;
}

export function LiveVideoPlayer({
  streamType,
  onStateChange,
  onStats,
  className,
}: LiveVideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const { getBasicAuthHeader } = useAuth();

  // Callbacks live in a ref so that a parent re-render with fresh inline
  // arrow functions does not tear down and rebuild the stream connection.
  const callbacks = useRef({ onStateChange, onStats });
  callbacks.current = { onStateChange, onStats };

  // getBasicAuthHeader is a useCallback in AuthProvider, but its identity
  // changes when credentials change; hold it in a ref for the same reason.
  const authRef = useRef(getBasicAuthHeader);
  authRef.current = getBasicAuthHeader;

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    let cancelled = false;
    let player: { destroy: () => void } | null = null;

    callbacks.current.onStateChange?.('connecting');

    void (async () => {
      const [{ default: mpegts }, authHeader] = await Promise.all([
        import('mpegts.js'),
        authRef.current(),
      ]);
      if (cancelled) return;

      if (!mpegts.isSupported()) {
        callbacks.current.onStateChange?.(
          'error',
          'This browser cannot play the stream (Media Source Extensions unavailable).',
        );
        return;
      }

      const instance = mpegts.createPlayer(
        { type: 'flv', isLive: true, url: buildFlvUrl(streamType) },
        {
          // Hand frames to MSE immediately rather than accumulating them first.
          enableStashBuffer: false,
          // Deliberately off: this accelerates playback to burn off buffer, the
          // same class of catch-up mechanism as the push.c stall ratchet that
          // caused the VLC late-pictures bug and was removed 2026-08-06.
          liveBufferLatencyChasing: false,
          ...(authHeader ? { headers: { Authorization: authHeader } } : {}),
        },
      );
      player = instance;

      instance.on(mpegts.Events.MEDIA_INFO, (info: Record<string, unknown>) => {
        callbacks.current.onStats?.({
          width: info.width as number | undefined,
          height: info.height as number | undefined,
          fps: info.fps as number | undefined,
          videoCodec: info.videoCodec as string | undefined,
        });
      });

      instance.on(mpegts.Events.STATISTICS_INFO, (stats: Record<string, unknown>) => {
        const speedKBps = (stats.speed as number | undefined) ?? 0;
        callbacks.current.onStats?.({
          bitrateKbps: Math.round(speedKBps * 8),
          droppedFrames: stats.droppedFrames as number | undefined,
        });
      });

      instance.on(mpegts.Events.ERROR, (type: string, detail: string) => {
        callbacks.current.onStateChange?.('error', describeError(type, detail));
      });

      instance.attachMediaElement(video);
      instance.load();
    })();

    return () => {
      cancelled = true;
      player?.destroy();
    };
  }, [streamType]);

  return (
    <video
      ref={videoRef}
      className={cn('h-full w-full bg-black object-contain', className)}
      data-testid="liveview-video"
      autoPlay
      muted
      playsInline
      onPlaying={() => callbacks.current.onStateChange?.('playing')}
      onWaiting={() => callbacks.current.onStateChange?.('stalled')}
    >
      <track kind="captions" />
    </video>
  );
}

/** Turn an mpegts.js error pair into something a human can act on. */
function describeError(type: string, detail: string): string {
  if (detail?.includes('401') || detail?.toLowerCase().includes('unauthorized')) {
    return 'The camera rejected these credentials for the video stream.';
  }
  if (type === 'NetworkError') {
    return 'Could not reach the video stream. The camera may not be streaming.';
  }
  return `Playback failed: ${detail || type}`;
}
```

Notes for the implementer:

- `autoPlay muted playsInline` uses the browser's own autoplay path rather than
  calling `.play()` by hand. Muted autoplay is always permitted; unmuted is not.
  There is likely no audio on this camera anyway.
- The `<track kind="captions" />` child is there to satisfy the
  `jsx-a11y/media-has-caption` lint rule. Do not delete it.

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/components/common/LiveVideoPlayer.test.tsx`
Expected: PASS, 6 tests.

**Step 5: Commit**

```bash
git add cross-compile/www/src/components/common/LiveVideoPlayer.tsx cross-compile/www/src/components/common/LiveVideoPlayer.test.tsx
git commit -m "feat(www): add LiveVideoPlayer backed by mpegts.js and MSE"
```

---

## Task 4: Connection state and retry UI

**Files:**
- Modify: `cross-compile/www/src/components/common/LiveVideoPlayer.tsx`
- Modify: `cross-compile/www/src/components/common/LiveVideoPlayer.test.tsx`

**Step 1: Write the failing tests**

Append to the existing `describe` block:

```tsx
  it('surfaces a credentials-specific message on a 401', async () => {
    const onStateChange = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStateChange={onStateChange} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const errorHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'error')?.[1];
    errorHandler('NetworkError', 'Unexpected status 401');

    expect(onStateChange).toHaveBeenCalledWith(
      'error',
      'The camera rejected these credentials for the video stream.',
    );
  });

  it('reports connecting before the player is ready', async () => {
    const onStateChange = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStateChange={onStateChange} />);

    expect(onStateChange).toHaveBeenCalledWith('connecting');
  });

  it('forwards decoded media info to onStats', async () => {
    const onStats = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStats={onStats} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const mediaHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'media_info')?.[1];
    mediaHandler({ width: 1280, height: 720, fps: 25, videoCodec: 'avc1.4d001f' });

    expect(onStats).toHaveBeenCalledWith(
      expect.objectContaining({ width: 1280, height: 720, fps: 25 }),
    );
  });

  it('converts the reported speed from KB/s to Kbps', async () => {
    const onStats = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStats={onStats} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const statsHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'statistics_info')?.[1];
    statsHandler({ speed: 128, droppedFrames: 3 });

    expect(onStats).toHaveBeenCalledWith(
      expect.objectContaining({ bitrateKbps: 1024, droppedFrames: 3 }),
    );
  });
```

**Step 2: Run to verify**

Run: `npx vitest run src/components/common/LiveVideoPlayer.test.tsx`
Expected: PASS — the Task 3 implementation already satisfies these. If any
fail, fix `LiveVideoPlayer.tsx`; do not weaken the tests.

**Step 3: Commit**

```bash
git add cross-compile/www/src/components/common/LiveVideoPlayer.test.tsx
git commit -m "test(www): cover LiveVideoPlayer state and stats reporting"
```

---

## Task 5: Replace the placeholder in LiveViewPage

**Files:**
- Modify: `cross-compile/www/src/pages/LiveViewPage.tsx:245-276`
- Modify: `cross-compile/www/src/pages/LiveViewPage.test.tsx`

**Step 1: Write the failing test**

Add the same `vi.mock('mpegts.js', ...)` block used in Task 3 to the top of
`LiveViewPage.test.tsx`, then add:

```tsx
  it('renders the video element instead of the old placeholder', async () => {
    renderWithProviders(<LiveViewPage />);

    expect(await screen.findByTestId('liveview-video')).toBeInTheDocument();
    expect(screen.queryByTestId('liveview-stream-preview-title')).not.toBeInTheDocument();
  });

  it('shows a retry affordance when playback fails', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    const errorHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'error')?.[1];
    errorHandler('NetworkError', 'connection refused');

    expect(await screen.findByTestId('liveview-retry-button')).toBeInTheDocument();
  });

  it('hides the LIVE indicator until playback actually starts', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    expect(screen.queryByTestId('liveview-live-indicator')).not.toBeInTheDocument();
  });
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/pages/LiveViewPage.test.tsx`
Expected: FAIL — no element with testid `liveview-video`.

**Step 3: Implement**

In `LiveViewPage.tsx`, add state near the other `useState` calls at line 46:

```tsx
  const [playerState, setPlayerState] = useState<PlayerState>('connecting');
  const [playerMessage, setPlayerMessage] = useState<string>();
  const [stats, setStats] = useState<StreamStats>({});
  const [playerKey, setPlayerKey] = useState(0);

  const handleStateChange = useCallback((state: PlayerState, message?: string) => {
    setPlayerState(state);
    setPlayerMessage(message);
  }, []);

  const handleStats = useCallback((next: StreamStats) => {
    setStats((prev) => ({ ...prev, ...next }));
  }, []);

  // Remounting the player is the retry: the effect cleanup destroys the old
  // instance and a fresh one reconnects.
  const handleRetry = useCallback(() => {
    setStats({});
    setPlayerKey((k) => k + 1);
  }, []);
```

Replace the placeholder block at lines 245–268 with:

```tsx
            {/* Live Video */}
            <div className="relative h-full w-full">
              <LiveVideoPlayer
                key={`${streamType}-${playerKey}`}
                streamType={streamType}
                onStateChange={handleStateChange}
                onStats={handleStats}
              />

              {playerState === 'connecting' && (
                <div
                  className="absolute inset-0 flex items-center justify-center bg-black/60 text-zinc-400"
                  data-testid="liveview-connecting"
                >
                  Connecting to stream…
                </div>
              )}

              {playerState === 'stalled' && (
                <div
                  className="absolute inset-0 flex items-center justify-center bg-black/60 text-zinc-400"
                  data-testid="liveview-stalled"
                >
                  Buffering…
                </div>
              )}

              {playerState === 'error' && (
                <div
                  className="absolute inset-0 flex flex-col items-center justify-center gap-4 bg-black/80 p-6 text-center"
                  data-testid="liveview-error"
                >
                  <p className="text-sm text-zinc-300">{playerMessage ?? 'Playback failed.'}</p>
                  <Button variant="outline" size="sm" onClick={handleRetry} data-testid="liveview-retry-button">
                    Retry
                  </Button>
                </div>
              )}
            </div>
```

Change the LIVE indicator at line 271 so it is conditional:

```tsx
            {playerState === 'playing' && (
              <div
                className="live-indicator absolute top-20 left-6 backdrop-blur-sm"
                data-testid="liveview-live-indicator"
              >
                LIVE
              </div>
            )}
```

Add the import:

```tsx
import { LiveVideoPlayer, type PlayerState, type StreamStats } from '@/components/common/LiveVideoPlayer';
```

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/pages/LiveViewPage.test.tsx`
Expected: PASS. Existing PTZ tests must still pass untouched.

**Step 5: Commit**

```bash
git add cross-compile/www/src/pages/LiveViewPage.tsx cross-compile/www/src/pages/LiveViewPage.test.tsx
git commit -m "feat(www): show live video in place of the LiveView placeholder"
```

---

## Task 6: Feed the stats cards real data

The cards already exist and look right. Only their data source changes. Two
rows are deleted rather than reconnected, because MSE exposes neither packet
loss nor round-trip latency and shipping a plausible fake number is worse than
shipping nothing.

**Files:**
- Modify: `cross-compile/www/src/pages/LiveViewPage.tsx:245-370`
- Modify: `cross-compile/www/src/pages/LiveViewPage.test.tsx`

**Step 1: Write the failing test**

```tsx
  it('fills the stream info card from decoded media info', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    const mediaHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'media_info')?.[1];
    mediaHandler({ width: 1280, height: 720, fps: 25, videoCodec: 'avc1.4d001f' });

    expect(await screen.findByTestId('liveview-resolution-value')).toHaveTextContent('1280x720');
    expect(screen.getByTestId('liveview-framerate-value')).toHaveTextContent('25 fps');
  });

  it('shows a placeholder rather than a fake number before stats arrive', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    expect(screen.getByTestId('liveview-resolution-value')).toHaveTextContent('—');
  });

  it('no longer displays unmeasurable packet loss and latency rows', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    expect(screen.queryByText('Packet Loss')).not.toBeInTheDocument();
    expect(screen.queryByText('Latency')).not.toBeInTheDocument();
  });
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/pages/LiveViewPage.test.tsx`
Expected: FAIL — resolution still reads `1920x1080`, and `Packet Loss` is present.

**Step 3: Implement**

Add a formatting helper above the component:

```tsx
/** Render a stat, or an em dash when the stream has not reported it yet. */
function stat(value: number | string | undefined, suffix = ''): string {
  return value === undefined ? '—' : `${value}${suffix}`;
}
```

Replace the hardcoded values:

| Location | Was | Becomes |
| --- | --- | --- |
| preview subtitle (was `:266`, now inside the video block) | `1920×1080 @ 30fps` | delete — the video itself is the preview now |
| Stream Info Resolution `:325` | `1920x1080` | `{stats.width ? \`${stats.width}x${stats.height}\` : '—'}` |
| Stream Info Bitrate | `4096 Kbps` | `{stat(stats.bitrateKbps, ' Kbps')}` |
| Stream Info Frame Rate | `30 fps` | `{stat(stats.fps, ' fps')}` |
| Stream Info Codec | `H.264` | `{stats.videoCodec ?? '—'}` |
| Network Stats Status | hardcoded Connected | `{playerState}` with colour keyed off `playerState === 'playing'` |
| Network Stats Packet Loss | `0.0%` | **delete both the label and value spans** |
| Network Stats Latency | `45 ms` | **delete both the label and value spans** |
| Network Stats Bandwidth | `4.2 Mbps` | `{stat(stats.bitrateKbps, ' Kbps')}` |
| — | — | **add** `Dropped frames` → `{stat(stats.droppedFrames)}` |

Add `data-testid="liveview-framerate-value"`, `liveview-bitrate-value`,
`liveview-codec-value`, and `liveview-dropped-frames-value` to the new value
spans. `liveview-resolution-value` already exists at line 328 — keep it.

Also update the `Connected` badge at line 241 to reflect `playerState`.

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/pages/LiveViewPage.test.tsx`
Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/www/src/pages/LiveViewPage.tsx cross-compile/www/src/pages/LiveViewPage.test.tsx
git commit -m "feat(www): drive LiveView stats cards from the real stream"
```

---

## Task 7: Real stream URL bar and working Copy button

**Files:**
- Modify: `cross-compile/www/src/pages/LiveViewPage.tsx:279-302`
- Modify: `cross-compile/www/src/pages/LiveViewPage.test.tsx`

**Step 1: Write the failing test**

```tsx
  it('shows the stream URL actually in use and copies it', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });
    const user = userEvent.setup();

    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    expect(screen.getByTestId('liveview-stream-url-value')).toHaveTextContent('/live/main.flv');

    await user.click(screen.getByTestId('liveview-copy-url-button'));
    expect(writeText).toHaveBeenCalledWith(expect.stringContaining('/live/main.flv'));
  });

  it('updates the stream URL when switching to the sub stream', async () => {
    const user = userEvent.setup();
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    await user.click(screen.getByTestId('liveview-sub-stream-button'));

    expect(screen.getByTestId('liveview-stream-url-value')).toHaveTextContent('/live/sub.flv');
  });
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run src/pages/LiveViewPage.test.tsx`
Expected: FAIL — the URL still reads `rtsp://192.168.1.100:554/main`.

**Step 3: Implement**

```tsx
  const streamUrl = buildFlvUrl(streamType);

  const handleCopyUrl = useCallback(() => {
    void navigator.clipboard.writeText(streamUrl).then(
      () => toast.success('Stream URL copied'),
      () => toast.error('Could not copy the stream URL'),
    );
  }, [streamUrl]);
```

Replace the hardcoded string at line 291 with `{streamUrl}`, and add
`onClick={handleCopyUrl}` to the button at line 293.

Import `buildFlvUrl` from `@/utils/streamUrl`.

**Step 4: Run test to verify it passes**

Run: `npx vitest run src/pages/LiveViewPage.test.tsx`
Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/www/src/pages/LiveViewPage.tsx cross-compile/www/src/pages/LiveViewPage.test.tsx
git commit -m "feat(www): show and copy the real stream URL"
```

---

## Task 8: Quality gates

**Step 1: Full test suite**

Run: `npm run test`
Expected: all pass, no unhandled rejections.

**Step 2: Lint**

Run: `npm run lint`
Expected: clean. The `--max-warnings 0` flag means warnings fail.

**Step 3: Types (both compilers)**

Run: `npm run type-check`
Expected: clean under both `tsc` and `tsc6`.

**Step 4: Production build and bundle check**

Run: `npm run build`
Expected: succeeds, writes to `SD_card_contents/anyka_hack/onvif/www`.

Then confirm mpegts.js landed in the lazy chunk, not the eager one:

```bash
ls -la ../../SD_card_contents/anyka_hack/onvif/www/js/ | grep -i liveview
```

Expected: the `LiveViewPage-*.js` chunk grew by roughly 180 KB (or a separate
`mpegts`-named chunk appeared), and `index-*.js` is essentially unchanged. **If
the main bundle grew instead, the dynamic import was hoisted — investigate
before continuing.**

**Step 5: Commit the rebuilt assets**

```bash
git add SD_card_contents/anyka_hack/onvif/www
git commit -m "build(www): rebuild WebUI assets with live video preview"
```

---

## Task 9: Hardware verification on 192.168.2.198

Nothing above proves a single frame decoded. Mocked tests cannot.

**Step 1: Run the dev server against the camera**

```bash
cd cross-compile/www && npm run dev
```

Open `http://localhost:3000`, log in, go to Live View.

**Step 2: Confirm and record**

- Video appears and moves.
- Stream Info shows a real resolution (expect `1280x720` on .198 per the
  `zt9101-cutover-blocked-on-127` note) and a plausible bitrate.
- Switching main ↔ sub reconnects and the picture changes.
- **Time from page open to first frame.** Record it. The design flags a risk
  that `process_header_phase` withholds the FLV header waiting for audio that
  this camera may never send. If startup exceeds ~3 s, that is the suspect —
  the fix is a constant in
  `streaming-lib/src/protocol/httpflv/httpflv.rs:21`, and it is **out of scope
  for this plan**. File it, do not fix it here.
- Leave the page open 5 minutes; confirm dropped frames stay flat and the
  picture does not drift or stall.

**Step 3: Verify on the device itself**

Deploy per @anyka-embedded-build, then load the WebUI from the camera's own
port 80 rather than the dev server. This is the only step that exercises the
production absolute cross-port URL and the real CORS header; the dev proxy
hides both.

**Step 4: Record the outcome**

Append measured startup time and resolution to the design doc's Risks section,
then commit.

---

## Out of scope

Fullscreen, snapshot-to-PNG, audio playback, and iOS Safari support.

Separately, `onvif/media/ops/capabilities.rs:32` advertises
`snapshot_uri: Some(true)` while nothing serves `/snapshot` — a real defect
worth its own issue, not part of this work.
