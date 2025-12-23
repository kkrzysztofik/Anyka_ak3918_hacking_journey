import React from 'react';

import {
  Activity,
  Clock,
  Cpu,
  Download,
  FileText,
  HardDrive,
  Info,
  Thermometer,
  Wifi,
} from 'lucide-react';
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';

// Mock data generator for charts
const generateData = (points: number, base: number, variance: number) => {
  return Array.from({ length: points }, (_, i) => ({
    time: i,
    value: Math.max(0, Math.min(100, base + (Math.random() - 0.5) * variance)),
  }));
};

const cpuData = generateData(30, 45, 15);
const memoryData = generateData(30, 60, 5);
const networkData = Array.from({ length: 30 }, (_, i) => ({
  time: i,
  upload: Math.max(0, 2 + (Math.random() - 0.5) * 1),
  download: Math.max(0, 4 + (Math.random() - 0.5) * 2),
}));

function StatCard({
  icon: Icon,
  label,
  value,
  subValue,
  color = 'text-muted-foreground',
  colorBg = 'bg-muted',
}: Readonly<{
  icon: React.ElementType;
  label: string;
  value: string;
  subValue?: string;
  color?: string;
  colorBg?: string;
}>) {
  return (
    <div className="border-border bg-card overflow-hidden rounded-xl border">
      <div className="flex items-center gap-4 p-5">
        <div
          className={cn('flex h-10 w-10 shrink-0 items-center justify-center rounded-lg', colorBg)}
        >
          <Icon className={cn('h-5 w-5', color)} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="text-muted-foreground text-sm font-medium">{label}</p>
          <h3 className="text-foreground font-mono text-xl font-bold">{value}</h3>
          {subValue && <p className="text-muted-foreground text-xs">{subValue}</p>}
        </div>
      </div>
    </div>
  );
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const CustomTooltip = ({ active, payload, label }: any) => {
  if (active && payload?.length) {
    return (
      <div className="border-border bg-card/95 text-foreground rounded-lg border p-2 text-xs shadow-xl backdrop-blur-md">
        <p className="text-muted-foreground mb-1 font-mono">{`Time: +${label}s`}</p>
        {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
        {payload.map((entry: any) => (
          <p key={`${entry.name}-${entry.value}`} style={{ color: entry.color }}>
            {entry.name}: {Number(entry.value).toFixed(1)}
            {entry.unit}
          </p>
        ))}
      </div>
    );
  }
  return null;
};

export default function DiagnosticsPage() {
  return (
    <div className="space-y-6 pb-8">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-bold text-white">Diagnostics & Statistics</h1>
        <p className="text-muted-foreground text-sm">
          Real-time system monitoring and performance metrics
        </p>
      </div>

      {/* Top Row: System Health Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StatCard
          icon={Activity}
          label="System Status"
          value="Healthy"
          subValue="● Online"
          color="text-green-500"
          colorBg="bg-green-500/10"
        />
        <StatCard
          icon={Cpu}
          label="CPU Usage"
          value="51%"
          subValue="Avg: 48%"
          color="text-red-500"
          colorBg="bg-red-500/10"
        />
        <StatCard
          icon={HardDrive}
          label="Memory"
          value="69%"
          subValue="1.4 GB / 2.0 GB"
          color="text-yellow-500"
          colorBg="bg-yellow-500/10"
        />
        <StatCard
          icon={Thermometer}
          label="Temperature"
          value="64°C"
          subValue="Normal range"
          color="text-blue-500"
          colorBg="bg-blue-500/10"
        />
      </div>

      {/* Charts Row 1: CPU & Memory */}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="border-border bg-card overflow-hidden">
          <CardHeader className="border-border border-b">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-red-500/10">
                  <Activity className="h-5 w-5 text-red-500" />
                </div>
                <div>
                  <CardTitle className="text-foreground text-sm font-semibold">CPU Usage</CardTitle>
                  <p className="text-muted-foreground text-xs">Processor load over time</p>
                </div>
              </div>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <Clock className="text-muted-foreground h-4 w-4" />
              </Button>
            </div>
          </CardHeader>
          <CardContent className="pt-4">
            <div className="h-[180px] w-full">
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={cpuData}>
                  <defs>
                    <linearGradient id="colorCpu" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#ef4444" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="#ef4444" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="#333" vertical={false} />
                  {/* Hidden Axis but keep domain */}
                  <XAxis dataKey="time" hide />
                  <YAxis domain={[0, 100]} hide />
                  <Tooltip
                    content={<CustomTooltip />}
                    cursor={{ stroke: '#333', strokeWidth: 1 }}
                  />
                  <Area
                    type="monotone"
                    dataKey="value"
                    stroke="#ef4444"
                    fillOpacity={1}
                    fill="url(#colorCpu)"
                    name="CPU"
                    unit="%"
                    strokeWidth={2}
                    isAnimationActive={false}
                  />
                </AreaChart>
              </ResponsiveContainer>
            </div>
            <div className="text-muted-foreground mt-2 flex justify-between font-mono text-xs">
              <span>00:00</span>
              <span>00:15</span>
              <span>00:30</span>
            </div>
          </CardContent>
        </Card>

        <Card className="border-border bg-card overflow-hidden">
          <CardHeader className="border-border border-b">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-yellow-500/10">
                  <HardDrive className="h-5 w-5 text-yellow-500" />
                </div>
                <div>
                  <CardTitle className="text-foreground text-sm font-semibold">
                    Memory Usage
                  </CardTitle>
                  <p className="text-muted-foreground text-xs">RAM utilization over time</p>
                </div>
              </div>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <Clock className="text-muted-foreground h-4 w-4" />
              </Button>
            </div>
          </CardHeader>
          <CardContent className="pt-4">
            <div className="h-[180px] w-full">
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={memoryData}>
                  <defs>
                    <linearGradient id="colorMem" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#eab308" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="#eab308" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="#333" vertical={false} />
                  <XAxis dataKey="time" hide />
                  <YAxis domain={[0, 100]} hide />
                  <Tooltip
                    content={<CustomTooltip />}
                    cursor={{ stroke: '#333', strokeWidth: 1 }}
                  />
                  <Area
                    type="monotone"
                    dataKey="value"
                    stroke="#eab308"
                    fillOpacity={1}
                    fill="url(#colorMem)"
                    name="Memory"
                    unit="%"
                    strokeWidth={2}
                    isAnimationActive={false}
                  />
                </AreaChart>
              </ResponsiveContainer>
            </div>
            <div className="text-muted-foreground mt-2 flex justify-between font-mono text-xs">
              <span>00:00</span>
              <span>00:15</span>
              <span>00:30</span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Charts Row 2: Network */}
      <Card className="border-border bg-card overflow-hidden">
        <CardHeader className="border-border border-b">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-blue-500/10">
                <Wifi className="h-5 w-5 text-blue-500" />
              </div>
              <div>
                <CardTitle className="text-foreground text-sm font-semibold">
                  Network Throughput
                </CardTitle>
                <p className="text-muted-foreground text-xs">Upload and download bandwidth</p>
              </div>
            </div>
            <div className="text-muted-foreground flex items-center gap-4 text-xs">
              <span className="flex items-center gap-1.5">
                <div className="h-2 w-2 rounded-full bg-blue-500"></div> Download
              </span>
              <span className="flex items-center gap-1.5">
                <div className="h-2 w-2 rounded-full bg-green-500"></div> Upload
              </span>
            </div>
          </div>
        </CardHeader>
        <CardContent className="pt-4">
          <div className="relative h-[120px] w-full">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={networkData}>
                <defs>
                  <linearGradient id="colorDown" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="colorUp" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#22c55e" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#22c55e" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" vertical={false} />
                <XAxis dataKey="time" hide />
                <YAxis hide />
                <Tooltip content={<CustomTooltip />} cursor={{ stroke: '#333', strokeWidth: 1 }} />
                <Area
                  type="monotone"
                  dataKey="download"
                  stroke="#3b82f6"
                  fillOpacity={1}
                  fill="url(#colorDown)"
                  name="Download"
                  unit=" Mbps"
                  strokeWidth={2}
                  isAnimationActive={false}
                />
                <Area
                  type="monotone"
                  dataKey="upload"
                  stroke="#22c55e"
                  fillOpacity={1}
                  fill="url(#colorUp)"
                  name="Upload"
                  unit=" Mbps"
                  strokeWidth={2}
                  isAnimationActive={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
          <div className="text-muted-foreground mt-2 flex justify-between font-mono text-xs">
            <span>00:00</span>
            <span>00:15</span>
            <span>00:30</span>
          </div>
        </CardContent>
      </Card>

      {/* Info Grid */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Device Info */}
        <Card className="border-border bg-card overflow-hidden">
          <CardHeader className="border-border border-b">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-blue-500/10">
                <Info className="h-5 w-5 text-blue-500" />
              </div>
              <div>
                <CardTitle className="text-foreground text-sm font-semibold">
                  Device Information
                </CardTitle>
                <p className="text-muted-foreground text-xs">Hardware and firmware details</p>
              </div>
            </div>
          </CardHeader>
          <CardContent className="pt-4">
            <dl className="grid grid-cols-2 gap-4 text-sm">
              <div className="space-y-1">
                <dt className="text-muted-foreground">Model</dt>
                <dd className="text-foreground font-mono">Anyka-3918-Pro</dd>
              </div>
              <div className="space-y-1 text-right">
                <dt className="text-muted-foreground">Firmware</dt>
                <dd className="text-foreground font-mono">v2.4.1</dd>
              </div>
              <div className="space-y-1">
                <dt className="text-muted-foreground">Serial Number</dt>
                <dd className="text-foreground font-mono">AK-2025-X892</dd>
              </div>
              <div className="space-y-1 text-right">
                <dt className="text-muted-foreground">IP Address</dt>
                <dd className="text-foreground font-mono">192.168.1.100</dd>
              </div>
              <div className="border-border col-span-2 mt-2 border-t pt-2">
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Uptime</span>
                  <span className="font-mono text-white">12d 5h 32m</span>
                </div>
              </div>
            </dl>
          </CardContent>
        </Card>

        {/* System Metrics */}
        <Card className="border-border bg-card overflow-hidden">
          <CardHeader className="border-border border-b">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-red-500/10">
                <Activity className="h-5 w-5 text-red-500" />
              </div>
              <div>
                <CardTitle className="text-foreground text-sm font-semibold">
                  System Metrics
                </CardTitle>
                <p className="text-muted-foreground text-xs">Performance and storage statistics</p>
              </div>
            </div>
          </CardHeader>
          <CardContent className="pt-4">
            <div className="space-y-4 text-sm">
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Storage Used</span>
                <span className="text-foreground font-mono">85% (42.5 GB / 50 GB)</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Active Streams</span>
                <span className="text-foreground font-mono">2</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Dropped Frames (24h)</span>
                <span className="text-foreground font-mono">12</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Avg Bitrate</span>
                <span className="text-foreground font-mono">4.2 Mbps</span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* System Logs */}
      <Card className="border-border bg-card overflow-hidden">
        <CardHeader className="border-border border-b">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-orange-500/10">
                <FileText className="h-5 w-5 text-orange-500" />
              </div>
              <div>
                <CardTitle className="text-foreground text-sm font-semibold">System Logs</CardTitle>
                <p className="text-muted-foreground text-xs">Recent activity and events</p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <div className="border-border bg-muted/50 flex items-center rounded-md border p-0.5">
                <Button variant="default" size="sm" className="h-6 px-2.5 text-xs">
                  All
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-muted-foreground hover:text-foreground h-6 px-2.5 text-xs"
                >
                  Info
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-muted-foreground hover:text-foreground h-6 px-2.5 text-xs"
                >
                  Warning
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-muted-foreground hover:text-foreground h-6 px-2.5 text-xs"
                >
                  Error
                </Button>
              </div>
              <Button variant="outline" size="sm" className="border-border h-7 gap-1 text-xs">
                <Download className="h-3 w-3" /> Export
              </Button>
            </div>
          </div>
        </CardHeader>
        <div className="max-h-[400px] overflow-x-auto overflow-y-auto">
          <table className="w-full text-left text-sm">
            <thead className="border-border bg-muted/40 text-muted-foreground border-b text-xs">
              <tr>
                <th className="px-5 py-3 font-medium">Timestamp</th>
                <th className="px-5 py-3 font-medium">Level</th>
                <th className="px-5 py-3 font-medium">Category</th>
                <th className="px-5 py-3 font-medium">Message</th>
              </tr>
            </thead>
            <tbody className="divide-border divide-y">
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="text-muted-foreground flex items-center gap-2">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">
                      2025-12-11
                      <br />
                      14:32:15
                    </span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-blue-500/30 bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-blue-400" aria-hidden="true"></span>
                    <span>Info</span>
                  </span>
                </td>
                <td className="text-foreground px-5 py-4 font-medium">Stream</td>
                <td className="text-muted-foreground px-5 py-4">
                  Main stream started successfully
                </td>
              </tr>
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="text-muted-foreground flex items-center gap-2">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">
                      2025-12-11
                      <br />
                      14:30:42
                    </span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-yellow-500/30 bg-yellow-500/10 px-2 py-0.5 text-xs font-medium text-yellow-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-yellow-400" aria-hidden="true"></span>
                    <span>Warning</span>
                  </span>
                </td>
                <td className="text-foreground px-5 py-4 font-medium">Network</td>
                <td className="text-muted-foreground px-5 py-4">High latency detected: 125ms</td>
              </tr>
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="text-muted-foreground flex items-center gap-2">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">
                      2025-12-11
                      <br />
                      14:28:03
                    </span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-blue-500/30 bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-blue-400" aria-hidden="true"></span>
                    <span>Info</span>
                  </span>
                </td>
                <td className="text-foreground px-5 py-4 font-medium">PTZ</td>
                <td className="text-muted-foreground px-5 py-4">Preset position 1 updated</td>
              </tr>
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="text-muted-foreground flex items-center gap-2">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">
                      2025-12-11
                      <br />
                      14:25:17
                    </span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-red-500/30 bg-red-500/10 px-2 py-0.5 text-xs font-medium text-red-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-red-400" aria-hidden="true"></span>
                    <span>Error</span>
                  </span>
                </td>
                <td className="text-foreground px-5 py-4 font-medium">Storage</td>
                <td className="text-muted-foreground px-5 py-4">Storage capacity at 85%</td>
              </tr>
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="text-muted-foreground flex items-center gap-2">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">
                      2025-12-11
                      <br />
                      14:20:55
                    </span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-blue-500/30 bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-blue-400" aria-hidden="true"></span>
                    <span>Info</span>
                  </span>
                </td>
                <td className="text-foreground px-5 py-4 font-medium">System</td>
                <td className="text-muted-foreground px-5 py-4">Firmware version 2.4.1 running</td>
              </tr>
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="text-muted-foreground flex items-center gap-2">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">
                      2025-12-11
                      <br />
                      14:18:33
                    </span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-blue-500/30 bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-blue-400" aria-hidden="true"></span>
                    <span>Info</span>
                  </span>
                </td>
                <td className="text-foreground px-5 py-4 font-medium">Auth</td>
                <td className="text-muted-foreground px-5 py-4">User admin logged in</td>
              </tr>
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}
