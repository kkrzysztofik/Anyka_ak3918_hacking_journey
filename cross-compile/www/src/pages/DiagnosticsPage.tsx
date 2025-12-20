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
  status = 'default',
  color = 'text-zinc-400',
}: {
  icon: React.ElementType;
  label: string;
  value: string;
  subValue?: string;
  status?: 'default' | 'success' | 'warning' | 'error';
  color?: string;
}) {
  const statusColors = {
    default: 'bg-zinc-900 border-zinc-800',
    success: 'bg-green-500/10 border-green-500/20',
    warning: 'bg-yellow-500/10 border-yellow-500/20',
    error: 'bg-red-500/10 border-red-500/20',
  };

  return (
    <Card className={cn('border', statusColors[status])}>
      <CardContent className="pt-6">
        <div className="flex items-start justify-between">
          <div>
            <p className="text-muted-foreground mb-1 text-sm font-medium">{label}</p>
            <h3 className="font-mono text-2xl font-bold">{value}</h3>
            {subValue && <p className="text-muted-foreground mt-1 text-xs">{subValue}</p>}
          </div>
          <div className={cn('rounded-lg bg-zinc-800/50 p-2', color)}>
            <Icon className="h-5 w-5" />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}



// eslint-disable-next-line @typescript-eslint/no-explicit-any
const CustomTooltip = ({ active, payload, label }: any) => {
  if (active && payload && payload.length) {
    return (
      <div className="rounded-lg border border-zinc-700 bg-zinc-900/90 p-2 text-xs text-zinc-100 shadow-xl backdrop-blur-md">
        <p className="mb-1 font-mono text-zinc-400">{`Time: +${label}s`}</p>
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
    <div className="space-y-6">
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
        <Card className="border-zinc-800 bg-zinc-900/40">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between">
              <CardTitle className="text-accent-red flex items-center gap-2 text-sm font-medium">
                <Activity className="h-4 w-4" />
                CPU Usage History
              </CardTitle>
              <Button variant="ghost" size="icon" className="h-6 w-6">
                <Clock className="h-3 w-3 text-zinc-500" />
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            <div className="h-[180px] w-full pt-4">
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
                  <Tooltip content={<CustomTooltip />} cursor={{ stroke: '#333', strokeWidth: 1 }} />
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
            <div className="mt-2 flex justify-between font-mono text-xs text-zinc-600">
              <span>00:00</span>
              <span>00:15</span>
              <span>00:30</span>
            </div>
          </CardContent>
        </Card>

        <Card className="border-zinc-800 bg-zinc-900/40">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between">
              <CardTitle className="flex items-center gap-2 text-sm font-medium text-yellow-500">
                <HardDrive className="h-4 w-4" />
                Memory Usage History
              </CardTitle>
              <Button variant="ghost" size="icon" className="h-6 w-6">
                <Clock className="h-3 w-3 text-zinc-500" />
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            <div className="h-[180px] w-full pt-4">
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
                  <Tooltip content={<CustomTooltip />} cursor={{ stroke: '#333', strokeWidth: 1 }} />
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
            <div className="mt-2 flex justify-between font-mono text-xs text-zinc-600">
              <span>00:00</span>
              <span>00:15</span>
              <span>00:30</span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Charts Row 2: Network */}
      <Card className="border-zinc-800 bg-zinc-900/40">
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="flex items-center gap-2 text-sm font-medium text-blue-500">
              <Wifi className="h-4 w-4" />
              Network Throughput
            </CardTitle>
            <div className="flex items-center gap-4 text-xs">
              <span className="flex items-center gap-1.5">
                <div className="h-2 w-2 rounded-full bg-blue-500"></div> Download
              </span>
              <span className="flex items-center gap-1.5">
                <div className="h-2 w-2 rounded-full bg-green-500"></div> Upload
              </span>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="relative h-[120px] w-full pt-4">
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
          <div className="mt-2 flex justify-between font-mono text-xs text-zinc-600">
            <span>00:00</span>
            <span>00:15</span>
            <span>00:30</span>
          </div>
        </CardContent>
      </Card>

      {/* Info Grid */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Device Info */}
        <Card className="border-zinc-800 bg-zinc-900/40">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-sm font-medium">
              <Info className="h-4 w-4 text-zinc-400" />
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
              <div className="col-span-2 mt-2 border-t border-zinc-800 pt-2">
                <div className="flex items-center justify-between">
                  <span className="text-zinc-500">Uptime</span>
                  <span className="font-mono text-white">12d 5h 32m</span>
                </div>
              </div>
            </dl>
          </CardContent>
        </Card>

        {/* System Metrics */}
        <Card className="border-zinc-800 bg-zinc-900/40">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-sm font-medium">
              <Activity className="h-4 w-4 text-red-500" />
              System Metrics
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4 text-sm">
              <div className="flex items-center justify-between">
                <span className="text-zinc-400">Storage Used</span>
                <span className="text-zinc-200">85% (42.5 GB / 50 GB)</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-zinc-400">Active Streams</span>
                <span className="text-zinc-200">2</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-zinc-400">Dropped Frames (24h)</span>
                <span className="text-zinc-200">12</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-zinc-400">Avg Bitrate</span>
                <span className="text-zinc-200">4.2 Mbps</span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* System Logs */}
      <Card className="overflow-hidden border-zinc-800 bg-zinc-900/40">
        <CardHeader className="border-b border-zinc-800 bg-zinc-900/50 px-4 py-3">
          <div className="flex items-center justify-between">
            <CardTitle className="text-accent-red flex items-center gap-2 text-sm font-medium">
              <FileText className="h-4 w-4" />
              System Logs
            </CardTitle>
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                className="h-7 border-zinc-700 bg-zinc-800 text-xs"
              >
                Filter
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-7 border-zinc-700 bg-zinc-800 text-xs"
              >
                <Download className="mr-1 h-3 w-3" /> Export
              </Button>
            </div>
          </div>
        </CardHeader>
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="bg-zinc-900/50 tracking-wider text-zinc-500 uppercase">
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
                <td className="px-4 py-3">
                  <span className="rounded border border-blue-900/30 bg-blue-900/20 px-1.5 py-0.5 text-blue-400">
                    Info
                  </span>
                </td>
                <td className="px-4 py-3 text-zinc-300">Stream</td>
                <td className="px-4 py-3 text-zinc-300">Main stream started successfully</td>
              </tr>
              <tr className="hover:bg-zinc-800/20">
                <td className="px-4 py-3 font-mono text-zinc-400">2025-12-15 14:30:42</td>
                <td className="px-4 py-3">
                  <span className="rounded border border-yellow-900/30 bg-yellow-900/20 px-1.5 py-0.5 text-yellow-400">
                    Warning
                  </span>
                </td>
                <td className="px-4 py-3 text-zinc-300">Network</td>
                <td className="px-4 py-3 text-zinc-300">High latency detected: 125ms</td>
              </tr>
              <tr className="hover:bg-zinc-800/20">
                <td className="px-4 py-3 font-mono text-zinc-400">2025-12-15 14:28:03</td>
                <td className="px-4 py-3">
                  <span className="rounded border border-blue-900/30 bg-blue-900/20 px-1.5 py-0.5 text-blue-400">
                    Info
                  </span>
                </td>
                <td className="px-4 py-3 text-zinc-300">PTZ</td>
                <td className="px-4 py-3 text-zinc-300">Preset position 1 updated</td>
              </tr>
              <tr className="hover:bg-zinc-800/20">
                <td className="px-4 py-3 font-mono text-zinc-400">2025-12-15 14:25:17</td>
                <td className="px-4 py-3">
                  <span className="rounded border border-red-900/30 bg-red-900/20 px-1.5 py-0.5 text-red-400">
                    Error
                  </span>
                </td>
                <td className="px-4 py-3 text-zinc-300">Storage</td>
                <td className="px-4 py-3 text-zinc-300">Storage capacity at 85%</td>
              </tr>
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}
