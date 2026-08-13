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
import { formatVideoCodec } from '@/utils/videoCodec';

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

  // Everything the effect needs but must not re-run for. Parents pass inline
  // arrows, and getBasicAuthHeader's identity changes with credentials; if the
  // effect depended on any of them, each parent render would tear down and
  // rebuild a live connection to the camera.
  const latest = useRef({ onStateChange, onStats, getBasicAuthHeader });
  useEffect(() => {
    latest.current = { onStateChange, onStats, getBasicAuthHeader };
  });

  // Last decoded-frame sample for measuring real fps (per-stream, reset below).
  const fpsSample = useRef<{ frames: number; at: number } | undefined>(undefined);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    let cancelled = false;
    let player: { destroy: () => void } | null = null;
    fpsSample.current = undefined;

    latest.current.onStateChange?.('connecting');

    void (async () => {
      try {
        const [{ default: mpegts }, authHeader] = await Promise.all([
          import('mpegts.js'),
          latest.current.getBasicAuthHeader(),
        ]);
        if (cancelled) return;

        if (!mpegts.isSupported()) {
          latest.current.onStateChange?.(
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

        // mpegts.js ships its own types (d.ts/mpegts.d.ts), so these payloads
        // arrive typed — no casts needed. MEDIA_INFO.fps is a guess from the
        // SPS VUI/default (23.976 when absent), so we ignore it and derive the
        // real frame rate from decodedFrames deltas in STATISTICS_INFO.
        instance.on(mpegts.Events.MEDIA_INFO, ({ width, height, videoCodec }) => {
          latest.current.onStats?.({
            width,
            height,
            videoCodec: formatVideoCodec(videoCodec),
          });
        });

        instance.on(mpegts.Events.STATISTICS_INFO, ({ speed, droppedFrames, decodedFrames }) => {
          // speed is KB/s; the UI shows Kbps.
          let fps: number | undefined;
          if (decodedFrames !== undefined) {
            const now = performance.now();
            const prev = fpsSample.current;
            if (prev && now > prev.at) {
              const measured = ((decodedFrames - prev.frames) / ((now - prev.at) / 1000)) | 0;
              if (measured > 0) fps = measured;
            }
            fpsSample.current = { frames: decodedFrames, at: now };
          }
          latest.current.onStats?.({
            bitrateKbps: Math.round((speed ?? 0) * 8),
            droppedFrames,
            ...(fps !== undefined ? { fps } : {}),
          });
        });

        instance.on(mpegts.Events.ERROR, (type, detail, info) => {
          latest.current.onStateChange?.('error', describeError(type, detail, info));
        });

        instance.attachMediaElement(video);
        instance.load();
      } catch (err) {
        // The dynamic import, auth lookup, or player construction can reject;
        // surface those as an error state instead of an unhandled rejection
        // that leaves the page stuck on "connecting" with no retry affordance.
        if (cancelled) return;
        latest.current.onStateChange?.(
          'error',
          err instanceof Error ? err.message : 'Could not start the video stream.',
        );
      }
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
      onPlaying={() => latest.current.onStateChange?.('playing')}
      onWaiting={() => latest.current.onStateChange?.('stalled')}
    />
  );
}

/** Turn an mpegts.js error triple into something a human can act on. */
function describeError(
  type: string,
  detail: string,
  info?: { code?: number },
): string {
  // mpegts.js puts the HTTP status on errorInfo.code; `detail` is a stable
  // ErrorDetails enum like "HttpStatusCodeInvalid" (no status digits).
  if (info?.code === 401 || detail?.includes('401') || detail?.toLowerCase().includes('unauthorized')) {
    return 'The camera rejected these credentials for the video stream.';
  }
  if (type === 'NetworkError') {
    return 'Could not reach the video stream. The camera may not be streaming.';
  }
  return `Playback failed: ${detail || type}`;
}
