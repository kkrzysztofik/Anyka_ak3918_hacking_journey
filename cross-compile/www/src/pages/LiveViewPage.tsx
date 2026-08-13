/**
 * Live View Page
 *
 * Video stream with real PTZ controls wired to ONVIF backend.
 */
import React, { useCallback, useRef, useState } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Activity,
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  Bookmark,
  Camera,
  Copy,
  Home,
  Settings2,
  Wifi,
} from 'lucide-react';
import { toast } from 'sonner';

import {
  LiveVideoPlayer,
  type PlayerState,
  type StreamStats,
} from '@/components/common/LiveVideoPlayer';
import { Button } from '@/components/ui/button';
import {
  SettingsCard,
  SettingsCardContent,
  SettingsCardDescription,
  SettingsCardHeader,
  SettingsCardTitle,
} from '@/components/ui/settings-card';
import { cn } from '@/lib/utils';
import { getProfiles } from '@/services/profileService';
import {
  continuousMove,
  getPresets,
  gotoHome,
  gotoPreset,
  removePreset,
  setPreset,
  stopMove,
} from '@/services/ptzService';
import type { PTZDirection, PTZPreset } from '@/services/ptzService';

/** States that cover the video with a message. Absent = show the picture. */
const WAITING_TEXT: Partial<Record<PlayerState, string>> = {
  connecting: 'Connecting to stream…',
  stalled: 'Buffering…',
};

export default function LiveViewPage() {
  const [streamType, setStreamType] = useState<'main' | 'sub'>('main');
  const [ptzSpeed, setPtzSpeed] = useState(50);
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
  const queryClient = useQueryClient();

  // Fetch profiles to get the PTZ-capable profile token
  const { data: profiles } = useQuery({
    queryKey: ['profiles'],
    queryFn: getProfiles,
  });

  const profileToken =
    profiles?.find((p) => p.ptzConfiguration)?.token ?? profiles?.[0]?.token ?? '';

  // Fetch presets for the active profile
  const { data: presets } = useQuery({
    queryKey: ['ptzPresets', profileToken],
    queryFn: () => getPresets(profileToken),
    enabled: !!profileToken,
  });

  // ContinuousMove mutation
  const moveMutation = useMutation({
    mutationFn: ({ direction }: { direction: PTZDirection }) =>
      continuousMove(profileToken, direction, ptzSpeed),
    onError: (error) => {
      toast.error('PTZ move failed', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  // Stop mutation
  const stopMutation = useMutation({
    mutationFn: () => stopMove(profileToken),
    onError: (error) => {
      toast.error('PTZ stop failed', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  // Home mutation
  const homeMutation = useMutation({
    mutationFn: () => gotoHome(profileToken),
    onError: (error) => {
      toast.error('Go to home failed', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  // Goto preset mutation
  const gotoPresetMutation = useMutation({
    mutationFn: (presetToken: string) => gotoPreset(profileToken, presetToken),
    onError: (error) => {
      toast.error('Go to preset failed', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  // Set preset mutation
  const setPresetMutation = useMutation({
    mutationFn: ({ name, presetToken }: { name: string; presetToken?: string }) =>
      setPreset(profileToken, name, presetToken),
    onSuccess: () => {
      toast.success('Preset saved');
      queryClient.invalidateQueries({ queryKey: ['ptzPresets', profileToken] });
    },
    onError: (error) => {
      toast.error('Save preset failed', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  // Remove preset mutation
  const removePresetMutation = useMutation({
    mutationFn: (presetToken: string) => removePreset(profileToken, presetToken),
    onSuccess: () => {
      toast.success('Preset removed');
      queryClient.invalidateQueries({ queryKey: ['ptzPresets', profileToken] });
    },
    onError: (error) => {
      toast.error('Remove preset failed', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  const isMovingRef = useRef(false);

  const handlePtzStart = useCallback(
    (direction: PTZDirection) => {
      if (!profileToken) return;
      isMovingRef.current = true;
      moveMutation.mutate({ direction });
    },
    [profileToken, moveMutation],
  );

  const handlePtzStop = useCallback(() => {
    if (!profileToken || !isMovingRef.current) return;
    isMovingRef.current = false;
    stopMutation.mutate();
  }, [profileToken, stopMutation]);

  const handleHome = useCallback(() => {
    if (!profileToken) return;
    homeMutation.mutate();
  }, [profileToken, homeMutation]);

  const handleGotoPreset = useCallback(
    (presetToken: string) => {
      if (!profileToken) return;
      gotoPresetMutation.mutate(presetToken);
    },
    [profileToken, gotoPresetMutation],
  );

  const handleAddPreset = useCallback(() => {
    if (!profileToken) return;
    const nextIndex = (presets?.length ?? 0) + 1;
    setPresetMutation.mutate({ name: `Preset ${nextIndex}` });
  }, [profileToken, presets, setPresetMutation]);

  const handleRemovePreset = useCallback(
    (presetToken: string) => {
      if (!profileToken) return;
      removePresetMutation.mutate(presetToken);
    },
    [profileToken, removePresetMutation],
  );

  // Build display presets: use real presets if available, fallback to placeholder slots
  const displayPresets: Array<{ index: number; preset?: PTZPreset }> = [1, 2, 3].map((i) => ({
    index: i,
    preset: presets?.[i - 1],
  }));

  return (
    <div className="flex h-full flex-col gap-6">
      <div className="flex shrink-0 flex-col gap-2">
        <h1 className="text-2xl font-medium text-white" data-testid="liveview-title">
          Live Video Preview
        </h1>
        <p className="text-muted-foreground text-sm" data-testid="liveview-description">
          Real-time ONVIF stream monitoring and playback controls
        </p>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-6 lg:grid-cols-[1fr_320px]">
        {/* Left Column: Video & Stream Info */}
        <div className="flex min-h-0 flex-col gap-4">
          {/* Main Video Area */}
          <div className="group relative flex-1 overflow-hidden rounded-xl border border-zinc-800 bg-black shadow-2xl">
            {/* Header Overlay */}
            <div className="absolute top-0 right-0 left-0 z-10 flex items-center justify-between bg-gradient-to-b from-black/80 to-transparent p-4 opacity-0 transition-opacity duration-300 group-hover:opacity-100">
              <div className="flex items-center gap-2 text-white/90">
                <Camera className="text-accent-red h-5 w-5" />
                <span className="font-medium">Video Stream</span>
              </div>
              <div className="flex items-center gap-2">
                {/* Semantic fieldset for stream type selection with reset styles */}
                <fieldset className="m-0 flex appearance-none rounded-lg border border-0 border-zinc-700/50 bg-zinc-900/80 p-0 p-1">
                  <legend className="sr-only">Stream type</legend>
                  <button
                    type="button"
                    onClick={() => setStreamType('main')}
                    className={cn(
                      'rounded-md px-3 py-1 text-xs font-medium transition-colors',
                      streamType === 'main'
                        ? 'bg-accent-red text-white'
                        : 'text-zinc-400 hover:text-white',
                    )}
                    aria-pressed={streamType === 'main'}
                    data-testid="liveview-main-stream-button"
                  >
                    Main Stream
                  </button>
                  <button
                    type="button"
                    onClick={() => setStreamType('sub')}
                    className={cn(
                      'rounded-md px-3 py-1 text-xs font-medium transition-colors',
                      streamType === 'sub'
                        ? 'bg-accent-red text-white'
                        : 'text-zinc-400 hover:text-white',
                    )}
                    aria-pressed={streamType === 'sub'}
                    data-testid="liveview-sub-stream-button"
                  >
                    Sub Stream
                  </button>
                </fieldset>
                <div className="status-badge-connected">Connected</div>
              </div>
            </div>

            {/* Live Video */}
            <div className="relative h-full w-full">
              <LiveVideoPlayer
                key={`${streamType}-${playerKey}`}
                streamType={streamType}
                onStateChange={handleStateChange}
                onStats={handleStats}
              />

              {WAITING_TEXT[playerState] && (
                <div
                  className="absolute inset-0 flex items-center justify-center bg-black/60 text-zinc-400"
                  data-testid={`liveview-${playerState}`}
                >
                  {WAITING_TEXT[playerState]}
                </div>
              )}

              {playerState === 'error' && (
                <div
                  className="absolute inset-0 flex flex-col items-center justify-center gap-4 bg-black/80 p-6 text-center"
                  data-testid="liveview-error"
                >
                  <p className="text-sm text-zinc-300">{playerMessage ?? 'Playback failed.'}</p>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleRetry}
                    data-testid="liveview-retry-button"
                  >
                    Retry
                  </Button>
                </div>
              )}
            </div>

            {playerState === 'playing' && (
              <div
                className="live-indicator absolute top-20 left-6 backdrop-blur-sm"
                data-testid="liveview-live-indicator"
              >
                LIVE
              </div>
            )}
          </div>

          {/* Stream URL Bar */}
          <div className="border-border bg-card flex items-center gap-4 rounded-lg border p-3">
            <span
              className="text-muted-foreground text-sm font-medium whitespace-nowrap"
              data-testid="liveview-stream-url-label"
            >
              Stream URL
            </span>
            <div
              className="border-border bg-background text-foreground flex-1 truncate rounded border px-3 py-2 font-mono text-sm"
              data-testid="liveview-stream-url-value"
            >
              rtsp://192.168.1.100:554/main
            </div>
            <Button
              variant="outline"
              size="sm"
              className="border-border bg-muted text-foreground hover:bg-muted/80"
              data-testid="liveview-copy-url-button"
            >
              <Copy className="mr-2 h-3.5 w-3.5" />
              Copy
            </Button>
          </div>

          {/* Bottom Info Grid */}
          <div className="grid grid-cols-2 gap-4">
            {/* Stream Info */}
            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-3">
                  <div className="bg-accent-red/10 flex h-10 w-10 items-center justify-center rounded-lg">
                    <Activity className="text-accent-red h-5 w-5" />
                  </div>
                  <div>
                    <SettingsCardTitle data-testid="liveview-stream-info-title">
                      Stream Info
                    </SettingsCardTitle>
                    <SettingsCardDescription>Video stream parameters</SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent>
                <div className="grid grid-cols-2 gap-y-3 text-sm">
                  <span className="text-muted-foreground" data-testid="liveview-resolution-label">
                    Resolution
                  </span>
                  <span
                    className="text-foreground text-right font-mono"
                    data-testid="liveview-resolution-value"
                  >
                    1920x1080
                  </span>
                  <span className="text-muted-foreground">Bitrate</span>
                  <span className="text-foreground text-right font-mono">4096 Kbps</span>
                  <span className="text-muted-foreground">Frame Rate</span>
                  <span className="text-foreground text-right font-mono">30 fps</span>
                  <span className="text-muted-foreground">Codec</span>
                  <span className="text-foreground text-right font-mono">H.264</span>
                </div>
              </SettingsCardContent>
            </SettingsCard>

            {/* Network Stats */}
            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-3">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-green-500/10">
                    <Wifi className="h-5 w-5 text-green-500" />
                  </div>
                  <div>
                    <SettingsCardTitle data-testid="liveview-network-stats-title">
                      Network Stats
                    </SettingsCardTitle>
                    <SettingsCardDescription>Connection metrics</SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent>
                <div className="grid grid-cols-2 gap-y-3 text-sm">
                  <span className="text-muted-foreground">Status</span>
                  <span className="flex items-center justify-end gap-1.5 text-right font-medium text-green-500">
                    <div className="status-pulse h-1.5 w-1.5 rounded-full bg-green-500" />
                    Connected
                  </span>
                  <span className="text-muted-foreground">Packet Loss</span>
                  <span className="text-foreground text-right font-mono">0.0%</span>
                  <span className="text-muted-foreground">Latency</span>
                  <span className="text-foreground text-right font-mono">45 ms</span>
                  <span className="text-muted-foreground">Bandwidth</span>
                  <span className="text-foreground text-right font-mono">4.2 Mbps</span>
                </div>
              </SettingsCardContent>
            </SettingsCard>
          </div>
        </div>

        {/* Right Column: Controls */}
        <div className="flex flex-col gap-4">
          {/* PTZ Controls */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-3">
                <div className="bg-accent-red/10 flex h-10 w-10 items-center justify-center rounded-lg">
                  <Camera className="text-accent-red h-5 w-5" />
                </div>
                <div>
                  <SettingsCardTitle data-testid="liveview-ptz-title">Pan & Tilt</SettingsCardTitle>
                  <SettingsCardDescription data-testid="liveview-ptz-description">
                    PTZ camera controls
                  </SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent>
              <div className="flex flex-col items-center gap-6">
                {/* D-Pad */}
                <div className="flex flex-col items-center gap-1">
                  {/* Top Row */}
                  <div className="flex gap-1">
                    <button
                      type="button"
                      onMouseDown={() => handlePtzStart('up-left')}
                      onMouseUp={handlePtzStop}
                      onMouseLeave={handlePtzStop}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          handlePtzStart('up-left');
                        }
                      }}
                      onKeyUp={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') handlePtzStop();
                      }}
                      aria-label="Pan up-left"
                      className="hover:bg-accent-red group flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                      data-testid="liveview-ptz-up-left-button"
                    >
                      <div className="-rotate-45">
                        <ArrowUp className="h-4 w-4 text-white md:h-5 md:w-5" />
                      </div>
                    </button>
                    <button
                      type="button"
                      onMouseDown={() => handlePtzStart('up')}
                      onMouseUp={handlePtzStop}
                      onMouseLeave={handlePtzStop}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          handlePtzStart('up');
                        }
                      }}
                      onKeyUp={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') handlePtzStop();
                      }}
                      aria-label="Pan up"
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                      data-testid="liveview-ptz-up-button"
                    >
                      <ArrowUp className="h-4 w-4 text-white md:h-5 md:w-5" />
                    </button>
                    <button
                      type="button"
                      onMouseDown={() => handlePtzStart('up-right')}
                      onMouseUp={handlePtzStop}
                      onMouseLeave={handlePtzStop}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          handlePtzStart('up-right');
                        }
                      }}
                      onKeyUp={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') handlePtzStop();
                      }}
                      aria-label="Pan up-right"
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                      data-testid="liveview-ptz-up-right-button"
                    >
                      <div className="rotate-45">
                        <ArrowUp className="h-4 w-4 text-white md:h-5 md:w-5" />
                      </div>
                    </button>
                  </div>

                  {/* Middle Row */}
                  <div className="flex gap-1">
                    <button
                      type="button"
                      onMouseDown={() => handlePtzStart('left')}
                      onMouseUp={handlePtzStop}
                      onMouseLeave={handlePtzStop}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          handlePtzStart('left');
                        }
                      }}
                      onKeyUp={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') handlePtzStop();
                      }}
                      aria-label="Pan left"
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                      data-testid="liveview-ptz-left-button"
                    >
                      <ArrowLeft className="h-4 w-4 text-white md:h-5 md:w-5" />
                    </button>
                    <button
                      type="button"
                      onClick={handleHome}
                      aria-label="Return to home position"
                      className="bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg shadow-lg shadow-red-900/20 transition-colors hover:bg-red-700 md:h-12 md:w-12"
                      data-testid="liveview-ptz-home-button"
                    >
                      <Home className="h-4 w-4 text-white md:h-5 md:w-5" />
                    </button>
                    <button
                      type="button"
                      onMouseDown={() => handlePtzStart('right')}
                      onMouseUp={handlePtzStop}
                      onMouseLeave={handlePtzStop}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          handlePtzStart('right');
                        }
                      }}
                      onKeyUp={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') handlePtzStop();
                      }}
                      aria-label="Pan right"
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                      data-testid="liveview-ptz-right-button"
                    >
                      <ArrowRight className="h-4 w-4 text-white md:h-5 md:w-5" />
                    </button>
                  </div>

                  {/* Bottom Row */}
                  <div className="flex gap-1">
                    <button
                      type="button"
                      onMouseDown={() => handlePtzStart('down-left')}
                      onMouseUp={handlePtzStop}
                      onMouseLeave={handlePtzStop}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          handlePtzStart('down-left');
                        }
                      }}
                      onKeyUp={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') handlePtzStop();
                      }}
                      aria-label="Pan down-left"
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                      data-testid="liveview-ptz-down-left-button"
                    >
                      <div className="-rotate-[135deg]">
                        <ArrowUp className="h-4 w-4 text-white md:h-5 md:w-5" />
                      </div>
                    </button>
                    <button
                      type="button"
                      onMouseDown={() => handlePtzStart('down')}
                      onMouseUp={handlePtzStop}
                      onMouseLeave={handlePtzStop}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          handlePtzStart('down');
                        }
                      }}
                      onKeyUp={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') handlePtzStop();
                      }}
                      aria-label="Pan down"
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                      data-testid="liveview-ptz-down-button"
                    >
                      <ArrowDown className="h-4 w-4 text-white md:h-5 md:w-5" />
                    </button>
                    <button
                      type="button"
                      onMouseDown={() => handlePtzStart('down-right')}
                      onMouseUp={handlePtzStop}
                      onMouseLeave={handlePtzStop}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          handlePtzStart('down-right');
                        }
                      }}
                      onKeyUp={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') handlePtzStop();
                      }}
                      aria-label="Pan down-right"
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                      data-testid="liveview-ptz-down-right-button"
                    >
                      <div className="rotate-[135deg]">
                        <ArrowUp className="h-4 w-4 text-white md:h-5 md:w-5" />
                      </div>
                    </button>
                  </div>
                </div>

                {/* Speed Slider */}
                <div className="w-full space-y-2">
                  <div className="flex justify-between text-xs text-zinc-500">
                    <span>Speed</span>
                    <span data-testid="liveview-ptz-speed-value">{ptzSpeed}%</span>
                  </div>
                  <input
                    type="range"
                    min="1"
                    max="100"
                    value={ptzSpeed}
                    onChange={(e) => setPtzSpeed(Number(e.target.value))}
                    className="accent-accent-red h-1.5 w-full cursor-pointer appearance-none rounded-lg bg-zinc-800"
                    data-testid="liveview-ptz-speed-slider"
                  />
                </div>
              </div>
            </SettingsCardContent>
          </SettingsCard>

          {/* Presets */}
          <SettingsCard className="flex-1">
            <SettingsCardHeader>
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-blue-500/10">
                  <Bookmark className="h-5 w-5 text-blue-500" />
                </div>
                <div>
                  <SettingsCardTitle data-testid="liveview-presets-title">
                    Presets
                  </SettingsCardTitle>
                  <SettingsCardDescription>Saved camera positions</SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent>
              <div className="space-y-2">
                {displayPresets.map(({ index, preset }) => (
                  <div key={index} className="group flex items-center gap-2">
                    <Button
                      variant="outline"
                      className="border-border bg-muted text-foreground hover:border-ring hover:bg-muted/80 flex-1 justify-between"
                      data-testid={`liveview-preset-${index}-button`}
                      onClick={() => preset && handleGotoPreset(preset.token)}
                    >
                      <span className="text-xs" data-testid={`liveview-preset-${index}-label`}>
                        {preset?.name || `Preset ${index}`}
                      </span>
                      <span className="bg-background text-muted-foreground rounded px-1.5 py-0.5 font-mono text-[10px]">
                        #{index}
                      </span>
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="text-muted-foreground hover:bg-muted hover:text-accent-red h-8 w-8"
                      data-testid={`liveview-preset-${index}-settings-button`}
                      onClick={() => preset && handleRemovePreset(preset.token)}
                    >
                      <Settings2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ))}
                <Button
                  variant="outline"
                  className="border-border text-muted-foreground hover:bg-muted/50 hover:text-foreground mt-2 w-full border-dashed"
                  data-testid="liveview-add-preset-button"
                  onClick={handleAddPreset}
                >
                  <span data-testid="liveview-add-preset-label">+ Add Preset</span>
                </Button>
              </div>
            </SettingsCardContent>
          </SettingsCard>
        </div>
      </div>
    </div>
  );
}
