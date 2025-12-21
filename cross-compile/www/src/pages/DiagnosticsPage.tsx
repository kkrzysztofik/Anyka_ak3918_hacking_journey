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
}: {
  icon: React.ElementType;
  label: string;
  value: string;
  subValue?: string;
  color?: string;
  colorBg?: string;
}) {
  return (
    <div className="overflow-hidden rounded-xl border border-border bg-card">
      <div className="flex items-center gap-4 p-5">
        <div className={cn('flex h-10 w-10 shrink-0 items-center justify-center rounded-lg', colorBg)}>
          <Icon className={cn('h-5 w-5', color)} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="text-muted-foreground text-sm font-medium">{label}</p>
          <h3 className="font-mono text-xl font-bold text-foreground">{value}</h3>
          {subValue && <p className="text-muted-foreground text-xs">{subValue}</p>}
        </div>
      </div>
    </div>
  );
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const CustomTooltip = ({ active, payload, label }: any) => {
  if (active && payload && payload.length) {
    return (
      <div className="rounded-lg border border-border bg-card/95 p-2 text-xs text-foreground shadow-xl backdrop-blur-md">
        <p className="mb-1 font-mono text-muted-foreground">{`Time: +${label}s`}</p>
        {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
        {payload.map((entry: any, index: number) => (
          <p key={index} style={{ color: entry.color }}>
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
        <Card className="overflow-hidden border-border bg-card">
          <CardHeader className="border-b border-border">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-red-500/10">
                  <Activity className="h-5 w-5 text-red-500" />
                </div>
                <div>
                  <CardTitle className="text-sm font-semibold text-foreground">CPU Usage</CardTitle>
                  <p className="text-xs text-muted-foreground">Processor load over time</p>
                </div>
              </div>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <Clock className="h-4 w-4 text-muted-foreground" />
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
            <div className="mt-2 flex justify-between font-mono text-xs text-muted-foreground">
              <span>00:00</span>
              <span>00:15</span>
              <span>00:30</span>
            </div>
          </CardContent>
        </Card>

        <Card className="overflow-hidden border-border bg-card">
          <CardHeader className="border-b border-border">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-yellow-500/10">
                  <HardDrive className="h-5 w-5 text-yellow-500" />
                </div>
                <div>
                  <CardTitle className="text-sm font-semibold text-foreground">Memory Usage</CardTitle>
                  <p className="text-xs text-muted-foreground">RAM utilization over time</p>
                </div>
              </div>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <Clock className="h-4 w-4 text-muted-foreground" />
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
            <div className="mt-2 flex justify-between font-mono text-xs text-muted-foreground">
              <span>00:00</span>
              <span>00:15</span>
              <span>00:30</span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Charts Row 2: Network */}
      <Card className="overflow-hidden border-border bg-card">
        <CardHeader className="border-b border-border">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-blue-500/10">
                <Wifi className="h-5 w-5 text-blue-500" />
              </div>
              <div>
                <CardTitle className="text-sm font-semibold text-foreground">Network Throughput</CardTitle>
                <p className="text-xs text-muted-foreground">Upload and download bandwidth</p>
              </div>
            </div>
            <div className="flex items-center gap-4 text-xs text-muted-foreground">
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
          <div className="mt-2 flex justify-between font-mono text-xs text-muted-foreground">
            <span>00:00</span>
            <span>00:15</span>
            <span>00:30</span>
          </div>
        </CardContent>
      </Card>

      {/* Info Grid */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Device Info */}
        <Card className="overflow-hidden border-border bg-card">
          <CardHeader className="border-b border-border">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-blue-500/10">
                <Info className="h-5 w-5 text-blue-500" />
              </div>
              <div>
                <CardTitle className="text-sm font-semibold text-foreground">Device Information</CardTitle>
                <p className="text-xs text-muted-foreground">Hardware and firmware details</p>
              </div>
            </div>
          </CardHeader>
          <CardContent className="pt-4">
            <dl className="grid grid-cols-2 gap-4 text-sm">
              <div className="space-y-1">
                <dt className="text-muted-foreground">Model</dt>
                <dd className="font-mono text-foreground">Anyka-3918-Pro</dd>
              </div>
              <div className="space-y-1 text-right">
                <dt className="text-muted-foreground">Firmware</dt>
                <dd className="font-mono text-foreground">v2.4.1</dd>
              </div>
              <div className="space-y-1">
                <dt className="text-muted-foreground">Serial Number</dt>
                <dd className="font-mono text-foreground">AK-2025-X892</dd>
              </div>
              <div className="space-y-1 text-right">
                <dt className="text-muted-foreground">IP Address</dt>
                <dd className="font-mono text-foreground">192.168.1.100</dd>
              </div>
              <div className="col-span-2 mt-2 border-t border-border pt-2">
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Uptime</span>
                  <span className="font-mono text-white">12d 5h 32m</span>
                </div>
              </div>
            </dl>
          </CardContent>
        </Card>

        {/* System Metrics */}
        <Card className="overflow-hidden border-border bg-card">
          <CardHeader className="border-b border-border">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-red-500/10">
                <Activity className="h-5 w-5 text-red-500" />
              </div>
              <div>
                <CardTitle className="text-sm font-semibold text-foreground">System Metrics</CardTitle>
                <p className="text-xs text-muted-foreground">Performance and storage statistics</p>
              </div>
            </div>
          </CardHeader>
          <CardContent className="pt-4">
            <div className="space-y-4 text-sm">
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Storage Used</span>
                <span className="font-mono text-foreground">85% (42.5 GB / 50 GB)</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Active Streams</span>
                <span className="font-mono text-foreground">2</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Dropped Frames (24h)</span>
                <span className="font-mono text-foreground">12</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Avg Bitrate</span>
                <span className="font-mono text-foreground">4.2 Mbps</span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* System Logs */}
      <Card className="overflow-hidden border-border bg-card">
        <CardHeader className="border-b border-border">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-orange-500/10">
                <FileText className="h-5 w-5 text-orange-500" />
              </div>
              <div>
                <CardTitle className="text-sm font-semibold text-foreground">System Logs</CardTitle>
                <p className="text-xs text-muted-foreground">Recent activity and events</p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <div className="flex items-center rounded-md border border-border bg-muted/50 p-0.5">
                <Button variant="default" size="sm" className="h-6 px-2.5 text-xs">
                  All
                </Button>
                <Button variant="ghost" size="sm" className="h-6 px-2.5 text-xs text-muted-foreground hover:text-foreground">
                  Info
                </Button>
                <Button variant="ghost" size="sm" className="h-6 px-2.5 text-xs text-muted-foreground hover:text-foreground">
                  Warning
                </Button>
                <Button variant="ghost" size="sm" className="h-6 px-2.5 text-xs text-muted-foreground hover:text-foreground">
                  Error
                </Button>
              </div>
              <Button variant="outline" size="sm" className="h-7 gap-1 border-border text-xs">
                <Download className="h-3 w-3" /> Export
              </Button>
            </div>
          </div>
        </CardHeader>
        <div className="max-h-[400px] overflow-y-auto overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead className="border-b border-border bg-muted/40 text-xs text-muted-foreground">
              <tr>
                <th className="px-5 py-3 font-medium">Timestamp</th>
                <th className="px-5 py-3 font-medium">Level</th>
                <th className="px-5 py-3 font-medium">Category</th>
                <th className="px-5 py-3 font-medium">Message</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">2025-12-11<br />14:32:15</span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-blue-500/30 bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-blue-400"></span>
                    Info
                  </span>
                </td>
                <td className="px-5 py-4 font-medium text-foreground">Stream</td>
                <td className="px-5 py-4 text-muted-foreground">Main stream started successfully</td>
              </tr>
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">2025-12-11<br />14:30:42</span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-yellow-500/30 bg-yellow-500/10 px-2 py-0.5 text-xs font-medium text-yellow-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-yellow-400"></span>
                    Warning
                  </span>
                </td>
                <td className="px-5 py-4 font-medium text-foreground">Network</td>
                <td className="px-5 py-4 text-muted-foreground">High latency detected: 125ms</td>
              </tr>
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">2025-12-11<br />14:28:03</span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-blue-500/30 bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-blue-400"></span>
                    Info
                  </span>
                </td>
                <td className="px-5 py-4 font-medium text-foreground">PTZ</td>
                <td className="px-5 py-4 text-muted-foreground">Preset position 1 updated</td>
              </tr>
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">2025-12-11<br />14:25:17</span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-red-500/30 bg-red-500/10 px-2 py-0.5 text-xs font-medium text-red-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-red-400"></span>
                    Error
                  </span>
                </td>
                <td className="px-5 py-4 font-medium text-foreground">Storage</td>
                <td className="px-5 py-4 text-muted-foreground">Storage capacity at 85%</td>
              </tr>
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">2025-12-11<br />14:20:55</span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-blue-500/30 bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-blue-400"></span>
                    Info
                  </span>
                </td>
                <td className="px-5 py-4 font-medium text-foreground">System</td>
                <td className="px-5 py-4 text-muted-foreground">Firmware version 2.4.1 running</td>
              </tr>
              <tr className="hover:bg-muted/30">
                <td className="px-5 py-4">
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Clock className="h-3.5 w-3.5" />
                    <span className="font-mono text-xs">2025-12-11<br />14:18:33</span>
                  </div>
                </td>
                <td className="px-5 py-4">
                  <span className="inline-flex items-center gap-1 rounded-full border border-blue-500/30 bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-400">
                    <span className="h-1.5 w-1.5 rounded-full bg-blue-400"></span>
                    Info
                  </span>
                </td>
                <td className="px-5 py-4 font-medium text-foreground">Auth</td>
                <td className="px-5 py-4 text-muted-foreground">User admin logged in</td>
              </tr>
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}
