/**
 * Live Video Player
 *
 * Plays the camera's HTTP-FLV stream. No browser decodes FLV natively, so
 * mpegts.js demuxes it in JS and remuxes to fMP4 for Media Source Extensions.
 * The library is loaded with a dynamic import so it stays inside the lazy
 * LiveViewPage chunk rather than the eager bundle.
 */
import React, { useEffect, useRef, useState } from 'react';

import { Volume2, VolumeX } from 'lucide-react';

import { useAuth } from '@/hooks/useAuth';
import { cn } from '@/lib/utils';
import { type StreamType, buildFlvUrl } from '@/utils/streamUrl';
import { formatAudioCodec, formatVideoCodec } from '@/utils/videoCodec';

export type PlayerState = 'connecting' | 'playing' | 'stalled' | 'error';

export interface StreamStats {
  readonly width?: number;
  readonly height?: number;
  readonly fps?: number;
  readonly videoCodec?: string;
  readonly bitrateKbps?: number;
  readonly droppedFrames?: number;
  readonly audioCodec?: string;
  readonly audioSampleRate?: number;
  readonly audioChannels?: number;
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

  // Starts muted and must: autoplay policies only let a muted element start on
  // its own. Unmuting later is allowed because the click is a user gesture.
  const [muted, setMuted] = useState(true);
  // Only offer the control when the stream actually carries a track — the sub
  // stream, or a camera with audio_enabled=false, has none.
  const [hasAudio, setHasAudio] = useState(false);

  // Viewer-local playback level, not mic gain: other clients are unaffected.
  const [volume, setVolume] = useState(1);

  // `muted` and `volume` are DOM properties, not attributes — React reflects
  // neither reliably after the initial render, so drive both from the element.
  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    video.muted = muted;
    video.volume = volume;
  }, [muted, volume]);

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
    setHasAudio(false);

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
        instance.on(mpegts.Events.MEDIA_INFO, (info) => {
          const { width, height, videoCodec, audioCodec } = info;
          // audioSampleRate/audioChannelCount are real fields on mpegts.js's
          // MediaInfo but reachable only through its index signature; narrow
          // them rather than casting, so a shape change degrades to "—".
          const sampleRate =
            typeof info.audioSampleRate === 'number' ? info.audioSampleRate : undefined;
          const channels =
            typeof info.audioChannelCount === 'number' ? info.audioChannelCount : undefined;

          setHasAudio(Boolean(audioCodec));
          latest.current.onStats?.({
            width,
            height,
            videoCodec: formatVideoCodec(videoCodec),
            audioCodec: formatAudioCodec(audioCodec),
            // Explicitly reset the audio metadata fields when absent: the sub
            // stream has no audio, and handleStats merges this update into the
            // previous one, so omitting the keys would keep the old sample rate
            // and channel count after a main→sub switch.
            audioSampleRate: sampleRate,
            audioChannels: channels,
          });
        });

        instance.on(mpegts.Events.STATISTICS_INFO, ({ speed, droppedFrames, decodedFrames }) => {
          // speed is KB/s; the UI shows Kbps.
          let fps: number | undefined;
          if (decodedFrames !== undefined) {
            const now = performance.now();
            const prev = fpsSample.current;
            if (prev && now > prev.at) {
              const measured = Math.trunc((decodedFrames - prev.frames) / ((now - prev.at) / 1000));
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
        if (player) {
          // attachMediaElement/load can reject after the instance exists; tear
          // it down here and clear the reference so unmount cleanup cannot
          // double-destroy it.
          try {
            player.destroy();
          } catch {
            // Swallow: the setup error below is what the operator needs.
          }
          player = null;
        }
        if (cancelled) return;
        latest.current.onStateChange?.(
          'error',
          err instanceof Error ? err.message : 'Could not start the video stream.',
        );
      }
    })();

    return () => {
      cancelled = true;
      if (player) {
        player.destroy();
        player = null;
      }
    };
  }, [streamType]);

  return (
    <div className={cn('relative h-full w-full', className)}>
      <video
        ref={videoRef}
        className="h-full w-full bg-black object-contain"
        data-testid="liveview-video"
        aria-label={`${streamType === 'main' ? 'Main Stream' : 'Sub Stream'} live video`}
        autoPlay
        muted={muted}
        playsInline
        onPlaying={() => latest.current.onStateChange?.('playing')}
        onWaiting={() => latest.current.onStateChange?.('stalled')}
      />

      {hasAudio && (
        <div className="absolute right-3 bottom-3 flex items-center gap-2 rounded-md bg-black/60 px-2 py-1.5">
          <button
            type="button"
            onClick={() => {
              // Unmuting into a zero volume is silence with no visible cause,
              // so a toggle out of mute restores an audible level.
              if (muted && volume === 0) setVolume(1);
              setMuted((previous) => !previous);
            }}
            className="rounded text-white transition-colors hover:text-zinc-300 focus-visible:ring-2 focus-visible:ring-white focus-visible:outline-none"
            aria-pressed={!muted}
            aria-label={muted ? 'Unmute audio' : 'Mute audio'}
            data-testid="liveview-mute-toggle"
          >
            {muted ? <VolumeX className="h-4 w-4" /> : <Volume2 className="h-4 w-4" />}
          </button>
          <input
            type="range"
            min={0}
            max={1}
            step={0.05}
            // Show what is audible, not what is stored: while muted that is zero.
            value={muted ? 0 : volume}
            onChange={(event) => {
              const next = Number(event.target.value);
              setVolume(next);
              setMuted(next === 0);
            }}
            className="h-1 w-20 cursor-pointer accent-white"
            aria-label="Volume"
            data-testid="liveview-volume-slider"
          />
        </div>
      )}
    </div>
  );
}

/** Turn an mpegts.js error triple into something a human can act on. */
function describeError(type: string, detail: string, info?: { code?: number }): string {
  // mpegts.js puts the HTTP status on errorInfo.code; `detail` is a stable
  // ErrorDetails enum like "HttpStatusCodeInvalid" (no status digits).
  if (
    info?.code === 401 ||
    detail?.includes('401') ||
    detail?.toLowerCase().includes('unauthorized')
  ) {
    return 'The camera rejected these credentials for the video stream.';
  }
  if (type === 'NetworkError') {
    return 'Could not reach the video stream. The camera may not be streaming.';
  }
  return `Playback failed: ${detail || type}`;
}
