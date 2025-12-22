/**
 * Live View Page
 *
 * Video stream and PTZ controls placeholder.
 */
import React, { useState } from 'react';

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

import { Button } from '@/components/ui/button';
import {
  SettingsCard,
  SettingsCardContent,
  SettingsCardDescription,
  SettingsCardHeader,
  SettingsCardTitle,
} from '@/components/ui/settings-card';
import { cn } from '@/lib/utils';

export default function LiveViewPage() {
  const [streamType, setStreamType] = useState<'main' | 'sub'>('main');
  const [ptzSpeed, setPtzSpeed] = useState(50);

  // Handlers for PTZ (placeholders)
  const handlePtz = (action: string) => console.log('PTZ:', action);

  return (
    <div className="flex h-full flex-col gap-6">
      <div className="flex shrink-0 flex-col gap-2">
        <h1 className="text-2xl font-medium text-white">Live Video Preview</h1>
        <p className="text-muted-foreground text-sm">
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
                <div className="flex rounded-lg border border-zinc-700/50 bg-zinc-900/80 p-1">
                  <button
                    onClick={() => setStreamType('main')}
                    className={cn(
                      'rounded-md px-3 py-1 text-xs font-medium transition-colors',
                      streamType === 'main'
                        ? 'bg-accent-red text-white'
                        : 'text-zinc-400 hover:text-white',
                    )}
                  >
                    Main Stream
                  </button>
                  <button
                    onClick={() => setStreamType('sub')}
                    className={cn(
                      'rounded-md px-3 py-1 text-xs font-medium transition-colors',
                      streamType === 'sub'
                        ? 'bg-accent-red text-white'
                        : 'text-zinc-400 hover:text-white',
                    )}
                  >
                    Sub Stream
                  </button>
                </div>
                <div className="status-badge-connected">Connected</div>
              </div>
            </div>

            {/* Video Placeholder */}
            <div className="flex h-full w-full flex-col items-center justify-center bg-[url('/grid-pattern.png')] bg-repeat text-zinc-500">
              <div className="mb-4 rounded-full bg-zinc-900/50 p-8">
                <Camera className="h-12 w-12 text-zinc-600" />
              </div>
              <h3 className="text-xl font-medium text-zinc-400">ONVIF Stream Preview</h3>
              <p className="text-sm text-zinc-600">1920×1080 @ 30fps</p>
            </div>

            {/* LIVE Indicator */}
            <div className="live-indicator absolute top-20 left-6 backdrop-blur-sm">LIVE</div>
          </div>

          {/* Stream URL Bar */}
          <div className="border-border bg-card flex items-center gap-4 rounded-lg border p-3">
            <span className="text-muted-foreground text-sm font-medium whitespace-nowrap">
              Stream URL
            </span>
            <div className="border-border bg-background text-foreground flex-1 truncate rounded border px-3 py-2 font-mono text-sm">
              rtsp://192.168.1.100:554/main
            </div>
            <Button
              variant="outline"
              size="sm"
              className="border-border bg-muted text-foreground hover:bg-muted/80"
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
                    <SettingsCardTitle>Stream Info</SettingsCardTitle>
                    <SettingsCardDescription>Video stream parameters</SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent>
                <div className="grid grid-cols-2 gap-y-3 text-sm">
                  <span className="text-muted-foreground">Resolution</span>
                  <span className="text-foreground text-right font-mono">1920x1080</span>
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
                    <SettingsCardTitle>Network Stats</SettingsCardTitle>
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
                  <SettingsCardTitle>Pan & Tilt</SettingsCardTitle>
                  <SettingsCardDescription>PTZ camera controls</SettingsCardDescription>
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
                      onMouseDown={() => handlePtz('up-left')}
                      className="hover:bg-accent-red group flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                    >
                      <div className="-rotate-45">
                        <ArrowUp className="h-4 w-4 text-white md:h-5 md:w-5" />
                      </div>
                    </button>
                    <button
                      onMouseDown={() => handlePtz('up')}
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                    >
                      <ArrowUp className="h-4 w-4 text-white md:h-5 md:w-5" />
                    </button>
                    <button
                      onMouseDown={() => handlePtz('up-right')}
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                    >
                      <div className="rotate-45">
                        <ArrowUp className="h-4 w-4 text-white md:h-5 md:w-5" />
                      </div>
                    </button>
                  </div>

                  {/* Middle Row */}
                  <div className="flex gap-1">
                    <button
                      onMouseDown={() => handlePtz('left')}
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                    >
                      <ArrowLeft className="h-4 w-4 text-white md:h-5 md:w-5" />
                    </button>
                    <button
                      onClick={() => handlePtz('home')}
                      className="bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg shadow-lg shadow-red-900/20 transition-colors hover:bg-red-700 md:h-12 md:w-12"
                    >
                      <Home className="h-4 w-4 text-white md:h-5 md:w-5" />
                    </button>
                    <button
                      onMouseDown={() => handlePtz('right')}
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                    >
                      <ArrowRight className="h-4 w-4 text-white md:h-5 md:w-5" />
                    </button>
                  </div>

                  {/* Bottom Row */}
                  <div className="flex gap-1">
                    <button
                      onMouseDown={() => handlePtz('down-left')}
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                    >
                      <div className="-rotate-[135deg]">
                        <ArrowUp className="h-4 w-4 text-white md:h-5 md:w-5" />
                      </div>
                    </button>
                    <button
                      onMouseDown={() => handlePtz('down')}
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
                    >
                      <ArrowDown className="h-4 w-4 text-white md:h-5 md:w-5" />
                    </button>
                    <button
                      onMouseDown={() => handlePtz('down-right')}
                      className="hover:bg-accent-red flex h-11 w-11 items-center justify-center rounded-lg bg-zinc-800 transition-colors active:bg-red-700 md:h-12 md:w-12"
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
                    <span>{ptzSpeed}%</span>
                  </div>
                  <input
                    type="range"
                    min="1"
                    max="100"
                    value={ptzSpeed}
                    onChange={(e) => setPtzSpeed(Number(e.target.value))}
                    className="accent-accent-red h-1.5 w-full cursor-pointer appearance-none rounded-lg bg-zinc-800"
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
                  <SettingsCardTitle>Presets</SettingsCardTitle>
                  <SettingsCardDescription>Saved camera positions</SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent>
              <div className="space-y-2">
                {[1, 2, 3].map((i) => (
                  <div key={i} className="group flex items-center gap-2">
                    <Button
                      variant="outline"
                      className="border-border bg-muted text-foreground hover:border-ring hover:bg-muted/80 flex-1 justify-between"
                    >
                      <span className="text-xs">Preset {i}</span>
                      <span className="bg-background text-muted-foreground rounded px-1.5 py-0.5 font-mono text-[10px]">
                        #{i}
                      </span>
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="text-muted-foreground hover:bg-muted hover:text-accent-red h-8 w-8"
                    >
                      <Settings2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ))}
                <Button
                  variant="outline"
                  className="border-border text-muted-foreground hover:bg-muted/50 hover:text-foreground mt-2 w-full border-dashed"
                >
                  + Add Preset
                </Button>
              </div>
            </SettingsCardContent>
          </SettingsCard>
        </div>
      </div>
    </div>
  );
}
