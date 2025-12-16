/**
 * Live View Page
 *
 * Video stream and PTZ controls placeholder.
 */

import React from 'react'
import { Camera, ZoomIn, ZoomOut, Move, Home, VideoOff } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

export default function LiveViewPage() {
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Live View</h1>

      <div className="grid gap-6 lg:grid-cols-[1fr_300px]">
        {/* Video Area */}
        <Card className="overflow-hidden">
          <div className="aspect-video bg-black flex items-center justify-center">
            <div className="text-center text-muted-foreground">
              <VideoOff className="w-16 h-16 mx-auto mb-4 opacity-50" />
              <h3 className="text-lg font-medium text-white/70">Stream Unavailable</h3>
              <p className="text-sm text-white/50 mt-1">
                Live video streaming is not yet implemented
              </p>
            </div>
          </div>
        </Card>

        {/* Controls */}
        <div className="space-y-4">
          {/* PTZ Controls */}
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-base flex items-center gap-2">
                <Move className="w-4 h-4" />
                PTZ Controls
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-3 gap-2">
                <div />
                <Button variant="outline" size="icon" disabled title="Tilt Up">
                  <span className="sr-only">Tilt Up</span>
                  ▲
                </Button>
                <div />

                <Button variant="outline" size="icon" disabled title="Pan Left">
                  <span className="sr-only">Pan Left</span>
                  ◀
                </Button>
                <Button variant="outline" size="icon" disabled title="Home">
                  <Home className="w-4 h-4" />
                </Button>
                <Button variant="outline" size="icon" disabled title="Pan Right">
                  <span className="sr-only">Pan Right</span>
                  ▶
                </Button>

                <div />
                <Button variant="outline" size="icon" disabled title="Tilt Down">
                  <span className="sr-only">Tilt Down</span>
                  ▼
                </Button>
                <div />
              </div>

              <div className="flex justify-center gap-4 mt-4">
                <Button variant="outline" size="icon" disabled title="Zoom Out">
                  <ZoomOut className="w-4 h-4" />
                </Button>
                <Button variant="outline" size="icon" disabled title="Zoom In">
                  <ZoomIn className="w-4 h-4" />
                </Button>
              </div>

              <p className="text-xs text-muted-foreground text-center mt-4">
                PTZ controls require an active stream
              </p>
            </CardContent>
          </Card>

          {/* Stream Info */}
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-base flex items-center gap-2">
                <Camera className="w-4 h-4" />
                Stream Info
              </CardTitle>
            </CardHeader>
            <CardContent>
              <dl className="text-sm space-y-2">
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Status</dt>
                  <dd className="text-destructive">Offline</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Resolution</dt>
                  <dd>--</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">FPS</dt>
                  <dd>--</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Bitrate</dt>
                  <dd>--</dd>
                </div>
              </dl>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}
