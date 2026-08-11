import React, { useState } from 'react';

import {
  Activity,
  Cpu,
  Database,
  Download,
  FileText,
  HardDrive,
  Info,
  Wifi,
} from 'lucide-react';

import { Sparkline } from '@/components/common/Sparkline';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useDiagnostics } from '@/hooks/useDiagnostics';
import { cn } from '@/lib/utils';

const RESTART_THRESHOLD_S = 300; // 5 minutes
const STALL_THRESHOLD_MS = 5000; // frame age above this → stalled

function formatDuration(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

function StatCard({
  icon: Icon,
  label,
  value,
  subValue,
  color = 'text-muted-foreground',
  colorBg = 'bg-muted',
  testId,
}: Readonly<{
  icon: React.ElementType;
  label: string;
  value: string;
  subValue?: string;
  color?: string;
  colorBg?: string;
  testId?: string;
}>) {
  const baseTestId = testId ?? `diagnostics-stat-${label.toLowerCase().replaceAll(/\s+/g, '-')}`;
  return (
    <div
      className="border-border bg-card overflow-hidden rounded-xl border"
      data-testid={baseTestId}
    >
      <div className="flex items-center gap-4 p-5">
        <div
          className={cn(
            'flex h-10 w-10 shrink-0 items-center justify-center rounded-lg',
            colorBg,
          )}
        >
          <Icon className={cn('h-5 w-5', color)} />
        </div>
        <div className="min-w-0 flex-1">
          <p
            className="text-muted-foreground text-sm font-medium"
            data-testid={`${baseTestId}-label`}
          >
            {label}
          </p>
          <h3
            className="text-foreground font-mono text-xl font-bold"
            data-testid={`${baseTestId}-value`}
          >
            {value}
          </h3>
          {subValue && (
            <p className="text-muted-foreground text-xs" data-testid={`${baseTestId}-subvalue`}>
              {subValue}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

export default function DiagnosticsPage() {
  const { data, isLoading, history } = useDiagnostics();
  const [logFilter, setLogFilter] = useState<'all' | 'info' | 'warning' | 'error'>('all');

  // Stat card values derived from real data
  const statusLabel =
    data === undefined ? '—' : data.status === 'ok' ? 'Healthy' : data.status;
  const statusColor = data?.status === 'ok' ? 'text-green-500' : 'text-yellow-500';
  const statusColorBg = data?.status === 'ok' ? 'bg-green-500/10' : 'bg-yellow-500/10';
  const statusSubValue =
    data === undefined
      ? undefined
      : data.degraded_services.length > 0
        ? `${data.degraded_services.length} service(s) degraded`
        : '● Online';

  const cpuValue =
    data?.cpu_percent !== null && data?.cpu_percent !== undefined
      ? `${Math.round(data.cpu_percent)}%`
      : '—';

  const memUsedMb = data?.memory ? Math.round(data.memory.used_kb / 1024) : null;
  const memTotalMb = data?.memory ? Math.round(data.memory.total_kb / 1024) : null;
  const memValue = memUsedMb !== null ? `${memUsedMb} MB` : '—';
  const memSubValue =
    memUsedMb !== null && memTotalMb !== null
      ? `${memUsedMb} MB / ${memTotalMb} MB`
      : undefined;

  const storageUsedMb = data?.storage ? Math.round(data.storage.used_kb / 1024) : null;
  const storageTotalMb = data?.storage ? Math.round(data.storage.total_kb / 1024) : null;
  const storageValue = storageUsedMb !== null ? `${storageUsedMb} MB` : '—';
  const storageSubValue =
    storageUsedMb !== null && storageTotalMb !== null
      ? `${storageUsedMb} MB / ${storageTotalMb} MB`
      : undefined;

  // Chart data from rolling history — nulls filtered before passing to Sparkline
  const cpuChartData = history
    .filter((p) => p.cpu !== null)
    .map((p) => ({ t: p.t, cpu: p.cpu as number }));

  const memChartData = history
    .filter((p) => p.memPct !== null)
    .map((p) => ({ t: p.t, memPct: p.memPct as number }));

  const netChartData = history
    .filter((p) => p.rx !== null && p.tx !== null)
    .map((p) => ({
      t: p.t,
      rxKbps: (p.rx as number) / 1000,
      txKbps: (p.tx as number) / 1000,
    }));

  // Uptime and restart detection
  const processUptime = data ? formatDuration(data.uptime.process_s) : '—';
  const systemUptime = data ? formatDuration(data.uptime.system_s) : '—';
  const restartGap = data ? data.uptime.system_s - data.uptime.process_s : 0;
  const hasRecentRestart = data !== undefined && restartGap > RESTART_THRESHOLD_S;

  if (isLoading) {
    return (
      <div className="flex h-64 items-center justify-center" data-testid="diagnostics-loading">
        <p className="text-muted-foreground text-sm">Loading diagnostics…</p>
      </div>
    );
  }

  return (
    <div className="space-y-6 pb-8">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-bold text-white" data-testid="diagnostics-title">
          Diagnostics & Statistics
        </h1>
        <p className="text-muted-foreground text-sm" data-testid="diagnostics-description">
          Real-time system monitoring and performance metrics
        </p>
      </div>

      {/* Stat cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StatCard
          icon={Activity}
          label="System Status"
          value={statusLabel}
          subValue={statusSubValue}
          color={statusColor}
          colorBg={statusColorBg}
          testId="diagnostics-stat-system-status"
        />
        <StatCard
          icon={Cpu}
          label="CPU Usage"
          value={cpuValue}
          color="text-red-500"
          colorBg="bg-red-500/10"
          testId="diagnostics-stat-cpu-usage"
        />
        <StatCard
          icon={HardDrive}
          label="Memory"
          value={memValue}
          subValue={memSubValue}
          color="text-yellow-500"
          colorBg="bg-yellow-500/10"
          testId="diagnostics-stat-memory"
        />
        <StatCard
          icon={Database}
          label="Storage"
          value={storageValue}
          subValue={storageSubValue}
          color="text-blue-500"
          colorBg="bg-blue-500/10"
          testId="diagnostics-stat-storage"
        />
      </div>

      {/* Charts Row 1: CPU & Memory */}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="border-border bg-card overflow-hidden">
          <CardHeader className="border-border border-b">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-red-500/10">
                <Activity className="h-5 w-5 text-red-500" />
              </div>
              <div>
                <CardTitle
                  className="text-foreground text-sm font-semibold"
                  data-testid="diagnostics-cpu-usage-title"
                >
                  CPU Usage
                </CardTitle>
                <p
                  className="text-muted-foreground text-xs"
                  data-testid="diagnostics-cpu-usage-description"
                >
                  Processor load over time
                </p>
              </div>
            </div>
          </CardHeader>
          <CardContent className="pt-4">
            <div className="h-[180px] w-full">
              {cpuChartData.length >= 2 ? (
                <Sparkline
                  data={cpuChartData}
                  series={[{ key: 'cpu', label: 'CPU', color: '#ef4444', unit: '%' }]}
                  domain={[0, 100]}
                />
              ) : (
                <div
                  className="flex h-full items-center justify-center"
                  data-testid="diagnostics-cpu-chart-empty"
                >
                  <p className="text-muted-foreground text-sm">Collecting data…</p>
                </div>
              )}
            </div>
          </CardContent>
        </Card>

        <Card className="border-border bg-card overflow-hidden">
          <CardHeader className="border-border border-b">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-yellow-500/10">
                <HardDrive className="h-5 w-5 text-yellow-500" />
              </div>
              <div>
                <CardTitle
                  className="text-foreground text-sm font-semibold"
                  data-testid="diagnostics-memory-usage-title"
                >
                  Memory Usage
                </CardTitle>
                <p
                  className="text-muted-foreground text-xs"
                  data-testid="diagnostics-memory-usage-description"
                >
                  RAM utilization over time
                </p>
              </div>
            </div>
          </CardHeader>
          <CardContent className="pt-4">
            <div className="h-[180px] w-full">
              {memChartData.length >= 2 ? (
                <Sparkline
                  data={memChartData}
                  series={[{ key: 'memPct', label: 'Memory', color: '#eab308', unit: '%' }]}
                  domain={[0, 100]}
                />
              ) : (
                <div
                  className="flex h-full items-center justify-center"
                  data-testid="diagnostics-memory-chart-empty"
                >
                  <p className="text-muted-foreground text-sm">Collecting data…</p>
                </div>
              )}
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
                <CardTitle
                  className="text-foreground text-sm font-semibold"
                  data-testid="diagnostics-network-throughput-title"
                >
                  Network Throughput
                </CardTitle>
                <p
                  className="text-muted-foreground text-xs"
                  data-testid="diagnostics-network-throughput-description"
                >
                  Upload and download bandwidth
                </p>
              </div>
            </div>
            <div className="text-muted-foreground flex items-center gap-4 text-xs">
              <span
                className="flex items-center gap-1.5"
                data-testid="diagnostics-network-download-label"
              >
                <div className="h-2 w-2 rounded-full bg-blue-500"></div> Download
              </span>
              <span
                className="flex items-center gap-1.5"
                data-testid="diagnostics-network-upload-label"
              >
                <div className="h-2 w-2 rounded-full bg-green-500"></div> Upload
              </span>
            </div>
          </div>
        </CardHeader>
        <CardContent className="pt-4">
          <div className="relative h-[120px] w-full">
            {netChartData.length >= 2 ? (
              <Sparkline
                data={netChartData}
                series={[
                  { key: 'rxKbps', label: 'Download', color: '#3b82f6', unit: ' kbps' },
                  { key: 'txKbps', label: 'Upload', color: '#22c55e', unit: ' kbps' },
                ]}
              />
            ) : (
              <div
                className="flex h-full items-center justify-center"
                data-testid="diagnostics-network-chart-empty"
              >
                <p className="text-muted-foreground text-sm">Collecting data…</p>
              </div>
            )}
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
                <CardTitle
                  className="text-foreground text-sm font-semibold"
                  data-testid="diagnostics-device-information-title"
                >
                  Device Information
                </CardTitle>
                <p
                  className="text-muted-foreground text-xs"
                  data-testid="diagnostics-device-information-description"
                >
                  Hardware and firmware details
                </p>
              </div>
            </div>
          </CardHeader>
          <CardContent className="pt-4">
            <dl className="space-y-3 text-sm">
              <div className="border-border border-t pt-3">
                <div
                  className="flex items-center justify-between"
                  data-testid="diagnostics-uptime-process-row"
                >
                  <span className="text-muted-foreground">Process Uptime</span>
                  <span
                    className="font-mono text-white"
                    data-testid="diagnostics-uptime-process"
                  >
                    {processUptime}
                  </span>
                </div>
                <div
                  className="mt-1 flex items-center justify-between"
                  data-testid="diagnostics-uptime-system-row"
                >
                  <span className="text-muted-foreground">System Uptime</span>
                  <span
                    className="font-mono text-white"
                    data-testid="diagnostics-uptime-system"
                  >
                    {systemUptime}
                  </span>
                </div>
                {hasRecentRestart && (
                  <p
                    className="mt-2 text-xs text-yellow-400"
                    data-testid="diagnostics-restart-note"
                  >
                    Restarted {formatDuration(restartGap)} ago
                  </p>
                )}
              </div>
            </dl>
          </CardContent>
        </Card>

        {/* Stream Health */}
        <Card className="border-border bg-card overflow-hidden">
          <CardHeader className="border-border border-b">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-red-500/10">
                <Activity className="h-5 w-5 text-red-500" />
              </div>
              <div>
                <CardTitle
                  className="text-foreground text-sm font-semibold"
                  data-testid="diagnostics-stream-health-title"
                >
                  Stream Health
                </CardTitle>
                <p
                  className="text-muted-foreground text-xs"
                  data-testid="diagnostics-stream-health-description"
                >
                  Video pipeline status
                </p>
              </div>
            </div>
          </CardHeader>
          <CardContent className="pt-4">
            <div className="space-y-3 text-sm">
              <div
                className="flex items-center justify-between"
                data-testid="diagnostics-frame-age"
              >
                <span className="text-muted-foreground">Frame Age</span>
                <span
                  className={cn(
                    'font-mono',
                    data?.stream_frame_age_ms !== null &&
                      data?.stream_frame_age_ms !== undefined &&
                      data.stream_frame_age_ms > STALL_THRESHOLD_MS
                      ? 'text-red-400'
                      : 'text-foreground',
                  )}
                >
                  {data?.stream_frame_age_ms !== null &&
                  data?.stream_frame_age_ms !== undefined
                    ? `${data.stream_frame_age_ms} ms`
                    : '—'}
                </span>
              </div>
              {data?.stream_frame_age_ms !== null &&
                data?.stream_frame_age_ms !== undefined &&
                data.stream_frame_age_ms > STALL_THRESHOLD_MS && (
                  <p className="text-xs text-red-400" data-testid="diagnostics-stream-stalled">
                    Stream stalled — no frame received in {data.stream_frame_age_ms} ms
                  </p>
                )}
              {data?.components && data.components.length > 0 && (
                <ul
                  className="mt-2 space-y-1"
                  data-testid="diagnostics-components-list"
                >
                  {data.components.map((c) => (
                    <li
                      key={c.name}
                      className="flex items-center justify-between"
                      data-testid={`diagnostics-component-${c.name}`}
                    >
                      <span className="text-muted-foreground">{c.name}</span>
                      <span
                        className={cn(
                          'font-mono text-xs',
                          c.status === 'ok' ? 'text-green-400' : 'text-yellow-400',
                        )}
                      >
                        {c.status}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
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
                <CardTitle
                  className="text-foreground text-sm font-semibold"
                  data-testid="diagnostics-system-logs-title"
                >
                  System Logs
                </CardTitle>
                <p
                  className="text-muted-foreground text-xs"
                  data-testid="diagnostics-system-logs-description"
                >
                  Recent activity and events
                </p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <div className="border-border bg-muted/50 flex items-center rounded-md border p-0.5">
                <Button
                  variant={logFilter === 'all' ? 'default' : 'ghost'}
                  size="sm"
                  className="h-6 px-2.5 text-xs"
                  data-testid="diagnostics-log-filter-all"
                  onClick={() => setLogFilter('all')}
                >
                  All
                </Button>
                <Button
                  variant={logFilter === 'info' ? 'default' : 'ghost'}
                  size="sm"
                  className="text-muted-foreground hover:text-foreground h-6 px-2.5 text-xs"
                  data-testid="diagnostics-log-filter-info"
                  onClick={() => setLogFilter('info')}
                >
                  Info
                </Button>
                <Button
                  variant={logFilter === 'warning' ? 'default' : 'ghost'}
                  size="sm"
                  className="text-muted-foreground hover:text-foreground h-6 px-2.5 text-xs"
                  data-testid="diagnostics-log-filter-warning"
                  onClick={() => setLogFilter('warning')}
                >
                  Warning
                </Button>
                <Button
                  variant={logFilter === 'error' ? 'default' : 'ghost'}
                  size="sm"
                  className="text-muted-foreground hover:text-foreground h-6 px-2.5 text-xs"
                  data-testid="diagnostics-log-filter-error"
                  onClick={() => setLogFilter('error')}
                >
                  Error
                </Button>
              </div>
              <Button
                variant="outline"
                size="sm"
                className="border-border h-7 gap-1 text-xs"
                data-testid="diagnostics-export-button"
              >
                <Download className="h-3 w-3" /> Export
              </Button>
            </div>
          </div>
        </CardHeader>
        <div className="max-h-[400px] overflow-y-auto" data-testid="diagnostics-log-panel">
          <p
            className="text-muted-foreground p-5 text-sm"
            data-testid="diagnostics-log-placeholder"
          >
            Select a log source to view entries.
          </p>
        </div>
      </Card>
    </div>
  );
}
