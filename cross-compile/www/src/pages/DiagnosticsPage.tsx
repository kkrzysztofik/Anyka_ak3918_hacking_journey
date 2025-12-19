
import React from 'react'
import { Activity, Cpu, HardDrive, Thermometer, Clock, Wifi, Info, FileText, Download } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

// Mock data generator for charts
const generateData = (points: number, base: number, variance: number) => {
  return Array.from({ length: points }, (_, i) => ({
    time: i,
    value: Math.max(0, Math.min(100, base + (Math.random() - 0.5) * variance))
  }))
}

const cpuData = generateData(30, 45, 15)
const memoryData = generateData(30, 60, 5)
const networkData = Array.from({ length: 30 }, (_, i) => ({
  time: i,
  upload: Math.max(0, 2 + (Math.random() - 0.5) * 1),
  download: Math.max(0, 4 + (Math.random() - 0.5) * 2)
}))

interface ChartDataPoint {
  time: number
  value: number
}

function SimpleLineChart({
  data,
  color,
  height = 60,
  showArea = true
}: {
  data: ChartDataPoint[]
  color: string
  height?: number
  showArea?: boolean
}) {
  const max = 100
  const points = data.map((d, i) => {
    const x = (i / (data.length - 1)) * 100
    const y = 100 - (d.value / max) * 100
    return `${x},${y}`
  }).join(' ')

  const areaPoints = `0,100 ${points} 100,100`

  const gradientId = `gradient-${color.replace('#', '')}`

  return (
    <div className="w-full h-full relative" style={{ height: `${height}px` }}>
      <svg viewBox="0 0 100 100" preserveAspectRatio="none" className="w-full h-full overflow-visible">
        <defs>
          <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={color} stopOpacity="0.2" />
            <stop offset="100%" stopColor={color} stopOpacity="0" />
          </linearGradient>
        </defs>
        {/* Grid lines */}
        <line x1="0" y1="0" x2="100" y2="0" stroke="#333" strokeWidth="0.5" strokeDasharray="2,2" />
        <line x1="0" y1="50" x2="100" y2="50" stroke="#333" strokeWidth="0.5" strokeDasharray="2,2" />
        <line x1="0" y1="100" x2="100" y2="100" stroke="#333" strokeWidth="0.5" />

        {showArea && (
          <polygon points={areaPoints} fill={`url(#${gradientId})`} />
        )}
        <polyline
          points={points}
          fill="none"
          stroke={color}
          strokeWidth="2"
          vectorEffect="non-scaling-stroke"
        />
      </svg>
    </div>
  )
}

function StatCard({
  icon: Icon,
  label,
  value,
  subValue,
  status = 'default',
  color = 'text-zinc-400'
}: {
  icon: React.ElementType
  label: string
  value: string
  subValue?: string
  status?: 'default' | 'success' | 'warning' | 'error'
  color?: string
}) {
  const statusColors = {
    default: 'bg-zinc-900 border-zinc-800',
    success: 'bg-green-500/10 border-green-500/20',
    warning: 'bg-yellow-500/10 border-yellow-500/20',
    error: 'bg-red-500/10 border-red-500/20'
  }

  return (
    <Card className={cn("border", statusColors[status])}>
      <CardContent className="pt-6">
        <div className="flex items-start justify-between">
          <div>
            <p className="text-sm font-medium text-muted-foreground mb-1">{label}</p>
            <h3 className="text-2xl font-bold font-mono">{value}</h3>
            {subValue && <p className="text-xs text-muted-foreground mt-1">{subValue}</p>}
          </div>
          <div className={cn("p-2 rounded-lg bg-zinc-800/50", color)}>
            <Icon className="w-5 h-5" />
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

export default function DiagnosticsPage() {
  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-bold text-white">Diagnostics & Statistics</h1>
        <p className="text-muted-foreground text-sm">Real-time system monitoring and performance metrics</p>
      </div>

      {/* Top Row: System Health Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StatCard
          icon={Activity}
          label="System Status"
          value="Healthy"
          subValue="● Online"
          status="success"
          color="text-green-500"
        />
        <StatCard
          icon={Cpu}
          label="CPU Usage"
          value="51%"
          subValue="Avg: 48%"
          status="default"
          color="text-red-500"
        />
        <StatCard
          icon={HardDrive}
          label="Memory"
          value="69%"
          subValue="1.4 GB / 2.0 GB"
          status="warning"
          color="text-yellow-500"
        />
        <StatCard
          icon={Thermometer}
          label="Temperature"
          value="64°C"
          subValue="Normal range"
          color="text-blue-500"
        />
      </div>

      {/* Charts Row 1: CPU & Memory */}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="bg-zinc-900/40 border-zinc-800">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm font-medium flex items-center gap-2 text-accent-red">
                <Activity className="w-4 h-4" />
                CPU Usage History
              </CardTitle>
              <Button variant="ghost" size="icon" className="h-6 w-6"><Clock className="w-3 h-3 text-zinc-500" /></Button>
            </div>
          </CardHeader>
          <CardContent>
            <div className="h-[180px] w-full pt-4">
              <SimpleLineChart data={cpuData} color="#ef4444" height={160} />
            </div>
            <div className="flex justify-between text-xs text-zinc-600 mt-2 font-mono">
              <span>00:00</span>
              <span>00:15</span>
              <span>00:30</span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-zinc-900/40 border-zinc-800">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm font-medium flex items-center gap-2 text-yellow-500">
                <HardDrive className="w-4 h-4" />
                Memory Usage History
              </CardTitle>
              <Button variant="ghost" size="icon" className="h-6 w-6"><Clock className="w-3 h-3 text-zinc-500" /></Button>
            </div>
          </CardHeader>
          <CardContent>
            <div className="h-[180px] w-full pt-4">
              <SimpleLineChart data={memoryData} color="#eab308" height={160} />
            </div>
            <div className="flex justify-between text-xs text-zinc-600 mt-2 font-mono">
              <span>00:00</span>
              <span>00:15</span>
              <span>00:30</span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Charts Row 2: Network */}
      <Card className="bg-zinc-900/40 border-zinc-800">
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm font-medium flex items-center gap-2 text-blue-500">
              <Wifi className="w-4 h-4" />
              Network Throughput
            </CardTitle>
            <div className="flex items-center gap-4 text-xs">
              <span className="flex items-center gap-1.5"><div className="w-2 h-2 rounded-full bg-blue-500"></div> Download</span>
              <span className="flex items-center gap-1.5"><div className="w-2 h-2 rounded-full bg-green-500"></div> Upload</span>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="h-[120px] w-full relative pt-4">
            {/* Double Chart Overlay */}
            <div className="absolute inset-0 pt-4">
              <svg viewBox="0 0 100 100" preserveAspectRatio="none" className="w-full h-full overflow-visible">
                <defs>
                  <linearGradient id="grad-download" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#3b82f6" stopOpacity="0.2" />
                    <stop offset="100%" stopColor="#3b82f6" stopOpacity="0" />
                  </linearGradient>
                  <linearGradient id="grad-upload" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#22c55e" stopOpacity="0.2" />
                    <stop offset="100%" stopColor="#22c55e" stopOpacity="0" />
                  </linearGradient>
                </defs>

                {/* Grid */}
                <line x1="0" y1="0" x2="100" y2="0" stroke="#333" strokeWidth="0.5" strokeDasharray="2,2" />
                <line x1="0" y1="50" x2="100" y2="50" stroke="#333" strokeWidth="0.5" strokeDasharray="2,2" />
                <line x1="0" y1="100" x2="100" y2="100" stroke="#333" strokeWidth="0.5" />

                {/* Download Area */}
                <polygon
                  points={`0,100 ${networkData.map((d, i) => {
                    const x = (i / (networkData.length - 1)) * 100
                    const y = 100 - (d.download / 10) * 100
                    return `${x},${y}`
                  }).join(' ')} 100,100`}
                  fill="url(#grad-download)"
                />

                {/* Upload Area */}
                <polygon
                  points={`0,100 ${networkData.map((d, i) => {
                    const x = (i / (networkData.length - 1)) * 100
                    const y = 100 - (d.upload / 10) * 100
                    return `${x},${y}`
                  }).join(' ')} 100,100`}
                  fill="url(#grad-upload)"
                />

                {/* Download Line */}
                <polyline
                  points={networkData.map((d, i) => {
                    const x = (i / (networkData.length - 1)) * 100
                    const y = 100 - (d.download / 10) * 100 // Scale max 10MB
                    return `${x},${y}`
                  }).join(' ')}
                  fill="none"
                  stroke="#3b82f6"
                  strokeWidth="2"
                  vectorEffect="non-scaling-stroke"
                />
                {/* Upload Line */}
                <polyline
                  points={networkData.map((d, i) => {
                    const x = (i / (networkData.length - 1)) * 100
                    const y = 100 - (d.upload / 10) * 100
                    return `${x},${y}`
                  }).join(' ')}
                  fill="none"
                  stroke="#22c55e"
                  strokeWidth="2"
                  vectorEffect="non-scaling-stroke"
                />
              </svg>
            </div>
          </div>
          <div className="flex justify-between text-xs text-zinc-600 mt-2 font-mono">
            <span>00:00</span>
            <span>00:15</span>
            <span>00:30</span>
          </div>
        </CardContent>
      </Card>

      {/* Info Grid */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Device Info */}
        <Card className="bg-zinc-900/40 border-zinc-800">
          <CardHeader>
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Info className="w-4 h-4 text-zinc-400" />
              Device Information
            </CardTitle>
          </CardHeader>
          <CardContent>
            <dl className="grid grid-cols-2 gap-4 text-sm">
              <div className="space-y-1">
                <dt className="text-zinc-500">Model</dt>
                <dd className="font-medium text-white">Anyka-3918-Pro</dd>
              </div>
              <div className="space-y-1 text-right">
                <dt className="text-zinc-500">Firmware</dt>
                <dd className="font-medium text-white">v2.4.1</dd>
              </div>
              <div className="space-y-1">
                <dt className="text-zinc-500">Serial Number</dt>
                <dd className="font-mono text-zinc-300">AK-2025-X892</dd>
              </div>
              <div className="space-y-1 text-right">
                <dt className="text-zinc-500">IP Address</dt>
                <dd className="font-mono text-zinc-300">192.168.1.100</dd>
              </div>
              <div className="col-span-2 pt-2 border-t border-zinc-800 mt-2">
                <div className="flex justify-between items-center">
                  <span className="text-zinc-500">Uptime</span>
                  <span className="font-mono text-white">12d 5h 32m</span>
                </div>
              </div>
            </dl>
          </CardContent>
        </Card>

        {/* System Metrics */}
        <Card className="bg-zinc-900/40 border-zinc-800">
          <CardHeader>
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Activity className="w-4 h-4 text-red-500" />
              System Metrics
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4 text-sm">
              <div className="flex justify-between items-center">
                <span className="text-zinc-400">Storage Used</span>
                <span className="text-zinc-200">85% (42.5 GB / 50 GB)</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-zinc-400">Active Streams</span>
                <span className="text-zinc-200">2</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-zinc-400">Dropped Frames (24h)</span>
                <span className="text-zinc-200">12</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-zinc-400">Avg Bitrate</span>
                <span className="text-zinc-200">4.2 Mbps</span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* System Logs */}
      <Card className="bg-zinc-900/40 border-zinc-800 overflow-hidden">
        <CardHeader className="py-3 px-4 border-b border-zinc-800 bg-zinc-900/50">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm font-medium flex items-center gap-2 text-accent-red">
              <FileText className="w-4 h-4" />
              System Logs
            </CardTitle>
            <div className="flex gap-2">
              <Button variant="outline" size="sm" className="h-7 text-xs bg-zinc-800 border-zinc-700">Filter</Button>
              <Button variant="outline" size="sm" className="h-7 text-xs bg-zinc-800 border-zinc-700"><Download className="w-3 h-3 mr-1" /> Export</Button>
            </div>
          </div>
        </CardHeader>
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="bg-zinc-900/50 text-zinc-500 uppercase tracking-wider">
              <tr>
                <th className="px-4 py-3 font-medium">Timestamp</th>
                <th className="px-4 py-3 font-medium">Level</th>
                <th className="px-4 py-3 font-medium">Category</th>
                <th className="px-4 py-3 font-medium">Message</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-zinc-800/50">
              <tr className="hover:bg-zinc-800/20">
                <td className="px-4 py-3 font-mono text-zinc-400">2025-12-15 14:32:15</td>
                <td className="px-4 py-3"><span className="text-blue-400 bg-blue-900/20 px-1.5 py-0.5 rounded border border-blue-900/30">Info</span></td>
                <td className="px-4 py-3 text-zinc-300">Stream</td>
                <td className="px-4 py-3 text-zinc-300">Main stream started successfully</td>
              </tr>
              <tr className="hover:bg-zinc-800/20">
                <td className="px-4 py-3 font-mono text-zinc-400">2025-12-15 14:30:42</td>
                <td className="px-4 py-3"><span className="text-yellow-400 bg-yellow-900/20 px-1.5 py-0.5 rounded border border-yellow-900/30">Warning</span></td>
                <td className="px-4 py-3 text-zinc-300">Network</td>
                <td className="px-4 py-3 text-zinc-300">High latency detected: 125ms</td>
              </tr>
              <tr className="hover:bg-zinc-800/20">
                <td className="px-4 py-3 font-mono text-zinc-400">2025-12-15 14:28:03</td>
                <td className="px-4 py-3"><span className="text-blue-400 bg-blue-900/20 px-1.5 py-0.5 rounded border border-blue-900/30">Info</span></td>
                <td className="px-4 py-3 text-zinc-300">PTZ</td>
                <td className="px-4 py-3 text-zinc-300">Preset position 1 updated</td>
              </tr>
              <tr className="hover:bg-zinc-800/20">
                <td className="px-4 py-3 font-mono text-zinc-400">2025-12-15 14:25:17</td>
                <td className="px-4 py-3"><span className="text-red-400 bg-red-900/20 px-1.5 py-0.5 rounded border border-red-900/30">Error</span></td>
                <td className="px-4 py-3 text-zinc-300">Storage</td>
                <td className="px-4 py-3 text-zinc-300">Storage capacity at 85%</td>
              </tr>
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  )
}
