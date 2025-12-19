/**
 * Live View Page
 *
 * Video stream and PTZ controls placeholder.
 */

import React, { useState } from 'react'
import {
  Camera,
  Move,
  Home,
  Settings2,
  Bookmark,
  ChevronDown,
  Wifi,
  Activity,
  Copy,
  ArrowUp,
  ArrowLeft,
  ArrowRight,
  ArrowDown
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { cn } from '@/lib/utils'

export default function LiveViewPage() {
  const [streamType, setStreamType] = useState<'main' | 'sub'>('main')
  const [ptzSpeed, setPtzSpeed] = useState(50)

  // Handlers for PTZ (placeholders)
  const handlePtz = (action: string) => console.log('PTZ:', action)

  return (
    <div className="flex flex-col gap-6 h-full">
      <div className="flex flex-col gap-2 shrink-0">
        <h1 className="text-2xl font-medium text-white">Live Video Preview</h1>
        <p className="text-muted-foreground text-sm">Real-time ONVIF stream monitoring and playback controls</p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[1fr_320px] gap-6 flex-1 min-h-0">
        {/* Left Column: Video & Stream Info */}
        <div className="flex flex-col gap-4 min-h-0">
          {/* Main Video Area */}
          <div className="relative flex-1 bg-black rounded-xl overflow-hidden group border border-zinc-800 shadow-2xl">
            {/* Header Overlay */}
            <div className="absolute top-0 left-0 right-0 p-4 flex items-center justify-between z-10 bg-gradient-to-b from-black/80 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300">
              <div className="flex items-center gap-2 text-white/90">
                <Camera className="w-5 h-5 text-accent-red" />
                <span className="font-medium">Video Stream</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="flex bg-zinc-900/80 rounded-lg p-1 border border-zinc-700/50">
                  <button
                    onClick={() => setStreamType('main')}
                    className={cn(
                      "px-3 py-1 text-xs font-medium rounded-md transition-colors",
                      streamType === 'main' ? "bg-accent-red text-white" : "text-zinc-400 hover:text-white"
                    )}
                  >
                    Main Stream
                  </button>
                  <button
                    onClick={() => setStreamType('sub')}
                    className={cn(
                      "px-3 py-1 text-xs font-medium rounded-md transition-colors",
                      streamType === 'sub' ? "bg-accent-red text-white" : "text-zinc-400 hover:text-white"
                    )}
                  >
                    Sub Stream
                  </button>
                </div>
                <div className="flex items-center gap-2 bg-zinc-900/80 px-3 py-1.5 rounded-lg border border-zinc-700/50">
                  <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
                  <span className="text-xs text-green-500 font-medium">Connected</span>
                </div>
              </div>
            </div>

            {/* Video Placeholder */}
            <div className="w-full h-full flex flex-col items-center justify-center text-zinc-500 bg-[url('/grid-pattern.png')] bg-repeat">
              <div className="p-8 rounded-full bg-zinc-900/50 mb-4">
                <Camera className="w-12 h-12 text-zinc-600" />
              </div>
              <h3 className="text-xl font-medium text-zinc-400">ONVIF Stream Preview</h3>
              <p className="text-sm text-zinc-600">1920×1080 @ 30fps</p>
            </div>

            {/* LIVE Indicator */}
            <div className="absolute top-20 left-6 flex items-center gap-2 bg-black/60 px-2 py-1 rounded-md backdrop-blur-sm">
              <div className="w-2 h-2 bg-red-600 rounded-full animate-pulse" />
              <span className="text-[10px] font-bold text-white tracking-widest">LIVE</span>
            </div>
          </div>

          {/* Stream URL Bar */}
          <div className="bg-zinc-900/50 p-3 rounded-lg border border-zinc-800 flex items-center gap-4">
            <span className="text-sm font-medium text-zinc-400 whitespace-nowrap">Stream URL</span>
            <div className="flex-1 bg-black/40 rounded px-3 py-2 text-sm font-mono text-zinc-300 truncate border border-zinc-800/50">
              rtsp://192.168.1.100:554/main
            </div>
            <Button variant="outline" size="sm" className="bg-zinc-800 border-zinc-700 text-zinc-300 hover:text-white hover:bg-zinc-700">
              <Copy className="w-3.5 h-3.5 mr-2" />
              Copy
            </Button>
          </div>

          {/* Bottom Info Grid */}
          <div className="grid grid-cols-2 gap-4">
            {/* Stream Info */}
            <Card className="bg-zinc-900/50 border-zinc-800 p-4">
              <div className="flex items-center gap-2 mb-4 text-accent-red">
                <Activity className="w-4 h-4" />
                <h3 className="font-semibold text-sm">Stream Info</h3>
              </div>
              <div className="grid grid-cols-2 gap-y-2 text-sm">
                <span className="text-zinc-500">Resolution</span>
                <span className="text-zinc-200 text-right font-mono">1920x1080</span>
                <span className="text-zinc-500">Bitrate</span>
                <span className="text-zinc-200 text-right font-mono">4096 Kbps</span>
                <span className="text-zinc-500">Frame Rate</span>
                <span className="text-zinc-200 text-right font-mono">30 fps</span>
                <span className="text-zinc-500">Codec</span>
                <span className="text-zinc-200 text-right font-mono">H.264</span>
              </div>
            </Card>

            {/* Network Stats */}
            <Card className="bg-zinc-900/50 border-zinc-800 p-4">
              <div className="flex items-center gap-2 mb-4 text-accent-red">
                <Wifi className="w-4 h-4" />
                <h3 className="font-semibold text-sm">Network Stats</h3>
              </div>
              <div className="grid grid-cols-2 gap-y-2 text-sm">
                <span className="text-zinc-500">Status</span>
                <span className="text-green-500 text-right font-medium flex justify-end items-center gap-1.5">
                  <div className="w-1.5 h-1.5 rounded-full bg-green-500" />
                  Connected
                </span>
                <span className="text-zinc-500">Packet Loss</span>
                <span className="text-zinc-200 text-right font-mono">0.0%</span>
                <span className="text-zinc-500">Latency</span>
                <span className="text-zinc-200 text-right font-mono">45 ms</span>
                <span className="text-zinc-500">Bandwidth</span>
                <span className="text-zinc-200 text-right font-mono">4.2 Mbps</span>
              </div>
            </Card>
          </div>
        </div>

        {/* Right Column: Controls */}
        <div className="flex flex-col gap-4">
          {/* PTZ Controls */}
          <Card className="bg-zinc-900/50 border-zinc-800 p-5">
            <div className="flex items-center gap-2 mb-6 text-accent-red">
              <Camera className="w-4 h-4" />
              <h3 className="font-semibold text-sm">Pan & Tilt</h3>
            </div>

            <div className="flex flex-col items-center gap-6">
              {/* D-Pad */}
              <div className="flex flex-col items-center gap-1">
                {/* Top Row */}
                <div className="flex gap-1">
                  <button
                    onMouseDown={() => handlePtz('up-left')}
                    className="w-11 h-11 md:w-12 md:h-12 rounded-lg bg-zinc-800 hover:bg-accent-red active:bg-red-700 flex items-center justify-center transition-colors group"
                  >
                    <div className="-rotate-45">
                      <ArrowUp className="w-4 h-4 md:w-5 md:h-5 text-white" />
                    </div>
                  </button>
                  <button
                    onMouseDown={() => handlePtz('up')}
                    className="w-11 h-11 md:w-12 md:h-12 rounded-lg bg-zinc-800 hover:bg-accent-red active:bg-red-700 flex items-center justify-center transition-colors"
                  >
                    <ArrowUp className="w-4 h-4 md:w-5 md:h-5 text-white" />
                  </button>
                  <button
                    onMouseDown={() => handlePtz('up-right')}
                    className="w-11 h-11 md:w-12 md:h-12 rounded-lg bg-zinc-800 hover:bg-accent-red active:bg-red-700 flex items-center justify-center transition-colors"
                  >
                    <div className="rotate-45">
                      <ArrowUp className="w-4 h-4 md:w-5 md:h-5 text-white" />
                    </div>
                  </button>
                </div>

                {/* Middle Row */}
                <div className="flex gap-1">
                  <button
                    onMouseDown={() => handlePtz('left')}
                    className="w-11 h-11 md:w-12 md:h-12 rounded-lg bg-zinc-800 hover:bg-accent-red active:bg-red-700 flex items-center justify-center transition-colors"
                  >
                    <ArrowLeft className="w-4 h-4 md:w-5 md:h-5 text-white" />
                  </button>
                  <button
                    onClick={() => handlePtz('home')}
                    className="w-11 h-11 md:w-12 md:h-12 rounded-lg bg-accent-red hover:bg-red-700 flex items-center justify-center transition-colors shadow-lg shadow-red-900/20"
                  >
                    <Home className="w-4 h-4 md:w-5 md:h-5 text-white" />
                  </button>
                  <button
                    onMouseDown={() => handlePtz('right')}
                    className="w-11 h-11 md:w-12 md:h-12 rounded-lg bg-zinc-800 hover:bg-accent-red active:bg-red-700 flex items-center justify-center transition-colors"
                  >
                    <ArrowRight className="w-4 h-4 md:w-5 md:h-5 text-white" />
                  </button>
                </div>

                {/* Bottom Row */}
                <div className="flex gap-1">
                  <button
                    onMouseDown={() => handlePtz('down-left')}
                    className="w-11 h-11 md:w-12 md:h-12 rounded-lg bg-zinc-800 hover:bg-accent-red active:bg-red-700 flex items-center justify-center transition-colors"
                  >
                    <div className="-rotate-[135deg]">
                      <ArrowUp className="w-4 h-4 md:w-5 md:h-5 text-white" />
                    </div>
                  </button>
                  <button
                    onMouseDown={() => handlePtz('down')}
                    className="w-11 h-11 md:w-12 md:h-12 rounded-lg bg-zinc-800 hover:bg-accent-red active:bg-red-700 flex items-center justify-center transition-colors"
                  >
                    <ArrowDown className="w-4 h-4 md:w-5 md:h-5 text-white" />
                  </button>
                  <button
                    onMouseDown={() => handlePtz('down-right')}
                    className="w-11 h-11 md:w-12 md:h-12 rounded-lg bg-zinc-800 hover:bg-accent-red active:bg-red-700 flex items-center justify-center transition-colors"
                  >
                    <div className="rotate-[135deg]">
                      <ArrowUp className="w-4 h-4 md:w-5 md:h-5 text-white" />
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
                  className="w-full h-1.5 bg-zinc-800 rounded-lg appearance-none cursor-pointer accent-accent-red"
                />
              </div>
            </div>
          </Card>

          {/* Presets */}
          <Card className="bg-zinc-900/50 border-zinc-800 p-5 flex-1">
            <div className="flex items-center gap-2 mb-4 text-accent-red">
              <Bookmark className="w-4 h-4" />
              <h3 className="font-semibold text-sm">Presets</h3>
            </div>
            <div className="space-y-2">
              {[1, 2, 3].map((i) => (
                <div key={i} className="flex items-center gap-2 group">
                  <Button variant="outline" className="flex-1 justify-between bg-zinc-800 border-zinc-700 text-zinc-300 hover:text-white hover:bg-zinc-700 hover:border-zinc-600">
                    <span className="text-xs">Preset {i}</span>
                    <span className="text-[10px] text-zinc-500 bg-black/20 px-1.5 py-0.5 rounded">#{i}</span>
                  </Button>
                  <Button variant="ghost" size="icon" className="w-8 h-8 text-zinc-500 hover:text-accent-red hover:bg-zinc-800">
                    <Settings2 className="w-3.5 h-3.5" />
                  </Button>
                </div>
              ))}
              <Button variant="outline" className="w-full mt-2 border-dashed border-zinc-700 text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50">
                + Add Preset
              </Button>
            </div>
          </Card>
        </div>
      </div>
    </div>
  )
}
