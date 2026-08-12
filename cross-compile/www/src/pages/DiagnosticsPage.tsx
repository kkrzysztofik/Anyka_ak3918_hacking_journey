import React, { useCallback, useState } from 'react';

import {
  Activity,
  Cpu,
  Database,
  Download,
  Eye,
  FileText,
  HardDrive,
  Info,
  Wifi,
} from 'lucide-react';

import { useQuery } from '@tanstack/react-query';

import { Sparkline } from '@/components/common/Sparkline';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useDiagnostics, type DiagnosticsPoint } from '@/hooks/useDiagnostics';
import { cn } from '@/lib/utils';
import {
  type Diagnostics,
  type LogLevel,
  type LogSource,
  getLogs,
} from '@/services/diagnosticsService';

const RESTART_THRESHOLD_S = 300; // 5 minutes
const STALL_THRESHOLD_MS = 5000; // frame age above this → stalled

const LOG_SOURCES: Array<{ value: LogSource; label: string }> = [
  { value: 'onvif_rust', label: 'ONVIF Service' },
  { value: 'vendor_daemon', label: 'Vendor Daemon' },
  { value: 'anyka_init', label: 'Anyka Init' },
  { value: 'wpa_supplicant', label: 'WPA Supplicant' },
];

const LOG_LEVEL_OPTIONS: Array<{ value: LogLevel | 'all'; label: string }> = [
  { value: 'all', label: 'All' },
  { value: 'info', label: 'Info' },
  { value: 'warn', label: 'Warning' },
  { value: 'error', label: 'Error' },
];

function formatKbps(bps: number): string {
  return `${Math.round(bps / 1000)} kbps`;
}

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

function formatLamp(supported: boolean, value: boolean | null | undefined): string {
  if (!supported) return 'n/a';
  if (value === null || value === undefined) return '\u2014';
  return value ? 'On' : 'Off';
}

function VisionCard({ vision }: Readonly<{ vision: Diagnostics['vision'] }>) {
  const mode = vision?.mode ?? '\u2014';
  const aeLuma =
    vision?.ae_luma !== null && vision?.ae_luma !== undefined
      ? String(vision.ae_luma)
      : '\u2014';
  const ain0 =
    vision?.ain0 !== null && vision?.ain0 !== undefined ? String(vision.ain0) : '\u2014';

  const irLed = vision ? formatLamp(vision.supported.ir_led, vision.ir_led) : '\u2014';
  const ircutA = vision ? formatLamp(vision.supported.ircut, vision.ircut_a) : '\u2014';
  const ircutB = vision ? formatLamp(vision.supported.ircut, vision.ircut_b) : '\u2014';
  const whiteLed = vision
    ? formatLamp(vision.supported.white_led, vision.white_led)
    : '\u2014';

  return (
    <Card className="border-border bg-card overflow-hidden">
      <CardHeader className="border-border border-b">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-purple-500/10">
            <Eye className="h-5 w-5 text-purple-500" />
          </div>
          <div>
            <CardTitle
              className="text-foreground text-sm font-semibold"
              data-testid="diagnostics-vision-title"
            >
              Day / Night Vision
            </CardTitle>
            <p className="text-muted-foreground text-xs">Ambient sensor and lamp state</p>
          </div>
        </div>
      </CardHeader>
      <CardContent className="pt-4">
        <dl className="space-y-2 text-sm">
          {[
            { label: 'Mode', value: mode, testId: 'diagnostics-vision-mode' },
            { label: 'AE luma', value: aeLuma, testId: 'diagnostics-vision-ae-luma' },
            { label: 'ain0', value: ain0, testId: 'diagnostics-vision-ain0' },
            { label: 'IR LED', value: irLed, testId: 'diagnostics-vision-ir-led' },
            { label: 'IR-CUT A', value: ircutA, testId: 'diagnostics-vision-ircut-a' },
            { label: 'IR-CUT B', value: ircutB, testId: 'diagnostics-vision-ircut-b' },
            { label: 'White LED', value: whiteLed, testId: 'diagnostics-vision-white-led' },
          ].map(({ label, value, testId }) => (
            <div key={testId} className="flex items-center justify-between">
              <dt className="text-muted-foreground">{label}</dt>
              <dd className="font-mono text-white" data-testid={testId}>
                {value}
              </dd>
            </div>
          ))}
        </dl>
      </CardContent>
    </Card>
  );
}

function formatStatusLabel(data: Diagnostics | undefined): string {
  if (data === undefined) return '—';
  if (data.status === 'healthy') return 'Healthy';
  return data.status;
}

function formatStatusSubValue(data: Diagnostics | undefined): string | undefined {
  if (data === undefined) return undefined;
  if (data.degraded_services.length > 0) {
    return `${data.degraded_services.length} service(s) degraded`;
  }
  return '● Online';
}

function ChartEmptyState({ testId }: Readonly<{ testId: string }>) {
  return (
    <div className="flex h-full items-center justify-center" data-testid={testId}>
      <p className="text-muted-foreground text-sm">Collecting data…</p>
    </div>
  );
}

function DiagnosticsStatCards({ data }: Readonly<{ data: Diagnostics | undefined }>) {
  const statusLabel = formatStatusLabel(data);
  const statusColor = data?.status === 'healthy' ? 'text-green-500' : 'text-yellow-500';
  const statusColorBg = data?.status === 'healthy' ? 'bg-green-500/10' : 'bg-yellow-500/10';
  const statusSubValue = formatStatusSubValue(data);

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

  return (
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
  );
}

function DiagnosticsChartsSection({ history }: Readonly<{ history: DiagnosticsPoint[] }>) {
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

  return (
    <>
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
            <div className="h-[200px] w-full">
              {cpuChartData.length >= 2 ? (
                <Sparkline
                  data={cpuChartData}
                  series={[{ key: 'cpu', label: 'CPU', color: '#ef4444', unit: '%' }]}
                  domain={[0, 100]}
                />
              ) : (
                <ChartEmptyState testId="diagnostics-cpu-chart-empty" />
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
            <div className="h-[200px] w-full">
              {memChartData.length >= 2 ? (
                <Sparkline
                  data={memChartData}
                  series={[{ key: 'memPct', label: 'Memory', color: '#eab308', unit: '%' }]}
                  domain={[0, 100]}
                />
              ) : (
                <ChartEmptyState testId="diagnostics-memory-chart-empty" />
              )}
            </div>
          </CardContent>
        </Card>
      </div>

      <Card className="border-border bg-card overflow-hidden">
        <CardHeader className="border-border border-b">
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
        </CardHeader>
        <CardContent className="pt-4">
          <div className="relative h-[160px] w-full">
            {netChartData.length >= 2 ? (
              <Sparkline
                data={netChartData}
                series={[
                  { key: 'rxKbps', label: 'Download', color: '#3b82f6', unit: ' kbps' },
                  { key: 'txKbps', label: 'Upload', color: '#22c55e', unit: ' kbps' },
                ]}
              />
            ) : (
              <ChartEmptyState testId="diagnostics-network-chart-empty" />
            )}
          </div>
        </CardContent>
      </Card>
    </>
  );
}

function DeviceInfoCard({ data }: Readonly<{ data: Diagnostics | undefined }>) {
  const processUptime = data ? formatDuration(data.uptime.process_s) : '—';
  const systemUptime = data ? formatDuration(data.uptime.system_s) : '—';
  const restartGap = data ? data.uptime.system_s - data.uptime.process_s : 0;
  const hasRecentRestart = data !== undefined && restartGap > RESTART_THRESHOLD_S;

  return (
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
              <dt className="text-muted-foreground">Process Uptime</dt>
              <dd className="font-mono text-white" data-testid="diagnostics-uptime-process">
                {processUptime}
              </dd>
            </div>
            <div
              className="mt-1 flex items-center justify-between"
              data-testid="diagnostics-uptime-system-row"
            >
              <dt className="text-muted-foreground">System Uptime</dt>
              <dd className="font-mono text-white" data-testid="diagnostics-uptime-system">
                {systemUptime}
              </dd>
            </div>
            {hasRecentRestart && data && (
              <p className="mt-2 text-xs text-yellow-400" data-testid="diagnostics-restart-note">
                Restarted {formatDuration(data.uptime.process_s)} ago
              </p>
            )}
          </div>
          <div className="border-border border-t pt-3">
            <div
              className="flex items-center justify-between"
              data-testid="diagnostics-network-download-row"
            >
              <dt className="text-muted-foreground">Download</dt>
              <dd className="font-mono text-white" data-testid="diagnostics-network-download">
                {data?.network != null ? formatKbps(data.network.rx_bps) : '\u2014'}
              </dd>
            </div>
            <div
              className="mt-1 flex items-center justify-between"
              data-testid="diagnostics-network-upload-row"
            >
              <dt className="text-muted-foreground">Upload</dt>
              <dd className="font-mono text-white" data-testid="diagnostics-network-upload">
                {data?.network != null ? formatKbps(data.network.tx_bps) : '\u2014'}
              </dd>
            </div>
          </div>
        </dl>
      </CardContent>
    </Card>
  );
}

function StreamHealthCard({ data }: Readonly<{ data: Diagnostics | undefined }>) {
  const frameAge = data?.stream_frame_age_ms;
  const hasFrameAge = frameAge !== null && frameAge !== undefined;
  const isStalled = hasFrameAge && frameAge > STALL_THRESHOLD_MS;

  return (
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
          <div className="flex items-center justify-between" data-testid="diagnostics-frame-age">
            <span className="text-muted-foreground">Frame Age</span>
            <span className={cn('font-mono', isStalled ? 'text-red-400' : 'text-foreground')}>
              {hasFrameAge ? `${frameAge} ms` : '—'}
            </span>
          </div>
          {isStalled && (
            <p className="text-xs text-red-400" data-testid="diagnostics-stream-stalled">
              Stream stalled — no frame received in {frameAge} ms
            </p>
          )}
          {data?.components && data.components.length > 0 && (
            <ul className="mt-2 space-y-1" data-testid="diagnostics-components-list">
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
                      c.status === 'healthy' ? 'text-green-400' : 'text-yellow-400',
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
  );
}

function LogPanelBody({
  logsFetching,
  logLines,
}: Readonly<{ logsFetching: boolean; logLines: string[] }>) {
  if (logsFetching && logLines.length === 0) {
    return (
      <p
        className="text-muted-foreground p-5 text-sm"
        data-testid="diagnostics-log-loading"
        aria-live="polite"
      >
        Loading…
      </p>
    );
  }
  if (logLines.length === 0) {
    return (
      <p
        className="text-muted-foreground p-5 text-sm"
        data-testid="diagnostics-log-unavailable"
        aria-live="polite"
      >
        Source unavailable or no log entries found.
      </p>
    );
  }
  return (
    <pre
      className="text-foreground divide-border divide-y p-4 font-mono text-xs leading-relaxed"
      data-testid="diagnostics-log-lines"
      aria-live="polite"
    >
      {logLines.join('\n')}
    </pre>
  );
}

function SystemLogsCard({
  logSource,
  setLogSource,
  logLevel,
  setLogLevel,
  logLines,
  logsFetching,
  onExport,
}: Readonly<{
  logSource: LogSource;
  setLogSource: (source: LogSource) => void;
  logLevel: LogLevel | 'all';
  setLogLevel: (level: LogLevel | 'all') => void;
  logLines: string[];
  logsFetching: boolean;
  onExport: () => void;
}>) {
  return (
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
            <Select value={logSource} onValueChange={(v) => setLogSource(v as LogSource)}>
              <SelectTrigger
                className="h-7 w-40 text-xs"
                data-testid="diagnostics-log-source-select"
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {LOG_SOURCES.map((s) => (
                  <SelectItem
                    key={s.value}
                    value={s.value}
                    data-testid={`diagnostics-log-source-option-${s.value}`}
                  >
                    {s.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>

            <fieldset
              className="border-border bg-muted/50 m-0 flex min-w-0 items-center rounded-md border p-0.5"
            >
              <legend className="sr-only">Log level filter</legend>
              {LOG_LEVEL_OPTIONS.map((opt) => (
                <Button
                  key={opt.value}
                  variant={logLevel === opt.value ? 'default' : 'ghost'}
                  size="sm"
                  className="h-6 px-2.5 text-xs"
                  data-testid={`diagnostics-log-filter-${opt.value}`}
                  aria-pressed={logLevel === opt.value}
                  onClick={() => setLogLevel(opt.value)}
                >
                  {opt.label}
                </Button>
              ))}
            </fieldset>

            <Button
              variant="outline"
              size="sm"
              className="border-border h-7 gap-1 text-xs"
              data-testid="diagnostics-export-button"
              onClick={onExport}
              disabled={logLines.length === 0}
            >
              <Download className="h-3 w-3" /> Export
            </Button>
          </div>
        </div>
      </CardHeader>
      <div className="max-h-[400px] overflow-y-auto" data-testid="diagnostics-log-panel">
        <LogPanelBody logsFetching={logsFetching} logLines={logLines} />
      </div>
    </Card>
  );
}

export default function DiagnosticsPage() {
  const { data, isLoading, history } = useDiagnostics();
  const [logSource, setLogSource] = useState<LogSource>('onvif_rust');
  const [logLevel, setLogLevel] = useState<LogLevel | 'all'>('all');

  const resolvedLevel = logLevel === 'all' ? undefined : logLevel;

  const { data: logLines = [], isFetching: logsFetching } = useQuery({
    queryKey: ['logs', logSource, resolvedLevel],
    queryFn: ({ signal }) => getLogs(logSource, resolvedLevel, 200, signal),
    staleTime: 10_000,
  });

  const handleExport = useCallback(() => {
    const blob = new Blob([logLines.join('\n')], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${logSource}.log`;
    a.click();
    // Defer revoke so Firefox can start the download before the blob URL is invalidated.
    globalThis.setTimeout(() => URL.revokeObjectURL(url), 0);
  }, [logLines, logSource]);

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

      <DiagnosticsStatCards data={data} />

      <DiagnosticsChartsSection history={history} />

      <div className="grid gap-6 lg:grid-cols-2">
        <DeviceInfoCard data={data} />
        <VisionCard vision={data?.vision ?? null} />
        <StreamHealthCard data={data} />
      </div>

      <SystemLogsCard
        logSource={logSource}
        setLogSource={setLogSource}
        logLevel={logLevel}
        setLogLevel={setLogLevel}
        logLines={logLines}
        logsFetching={logsFetching}
        onExport={handleExport}
      />
    </div>
  );
}
