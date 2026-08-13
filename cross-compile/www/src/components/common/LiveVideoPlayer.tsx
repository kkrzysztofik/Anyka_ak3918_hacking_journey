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

  // Everything the effect needs but must not re-run for. Parents pass inline
  // arrows, and getBasicAuthHeader's identity changes with credentials; if the
  // effect depended on any of them, each parent render would tear down and
  // rebuild a live connection to the camera.
  const latest = useRef({ onStateChange, onStats, getBasicAuthHeader });
  useEffect(() => {
    latest.current = { onStateChange, onStats, getBasicAuthHeader };
  });

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    let cancelled = false;
    let player: { destroy: () => void } | null = null;

    latest.current.onStateChange?.('connecting');

    void (async () => {
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
      // arrive typed — no casts needed.
      instance.on(mpegts.Events.MEDIA_INFO, ({ width, height, fps, videoCodec }) => {
        latest.current.onStats?.({ width, height, fps, videoCodec });
      });

      instance.on(mpegts.Events.STATISTICS_INFO, ({ speed, droppedFrames }) => {
        // speed is KB/s; the UI shows Kbps.
        latest.current.onStats?.({ bitrateKbps: Math.round((speed ?? 0) * 8), droppedFrames });
      });

      instance.on(mpegts.Events.ERROR, (type, detail) => {
        latest.current.onStateChange?.('error', describeError(type, detail));
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
      onPlaying={() => latest.current.onStateChange?.('playing')}
      onWaiting={() => latest.current.onStateChange?.('stalled')}
    />
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
