/**
 * Processes card for the Diagnostics page.
 *
 * Two lists from one endpoint, one 10 s poll:
 * - supervised services (anyka-init control socket) — the actionable set,
 *   each restartable through a confirm dialog;
 * - all processes (raw /proc walk) — collapsed by default, for diagnosis.
 */
import { useCallback, useEffect, useRef, useState } from 'react';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Activity, ChevronDown, RotateCw } from 'lucide-react';
import { toast } from 'sonner';

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { isAbortError, waitForCameraBack } from '@/lib/waitForCameraBack';
import { getDiagnostics } from '@/services/diagnosticsService';
import {
  type ServiceAction,
  type ServiceStatus,
  getProcesses,
  restartService,
  serviceAction,
} from '@/services/processesService';
import { formatDuration } from '@/utils/formatDuration';

// One endpoint feeds both tables; split queries would double requests to save nothing.
const REFRESH_MS = 10_000;
// After a POST that drops the connection the camera needs a full reboot cycle.
const ONVIF_WAIT_INTERVAL_MS = 2000;
const ONVIF_WAIT_TIMEOUT_MS = 5 * 60 * 1000;

/// Services the supervisor refuses to toggle (anyka-init: NON_TOGGLEABLE).
/// Rendering a button that always 404s is worse than rendering none.
const NON_TOGGLEABLE = new Set(['wpa_supplicant']);

const DISABLE_COPY: Record<string, string> = {
  onvif:
    'Web, ONVIF and RTSP access end immediately. The camera stays up, but it is then only reachable via FTP (or the deadman telnet after a failed boot). Re-enabling requires FTP or an SD-card edit.',
  'vendor-daemon':
    'Video capture and encoding stop; streams go dead until it is re-enabled. The camera does not reboot — the video watchdog is stood down with it.',
  udhcpc:
    'Stops DHCP renewals, so a configured static address is no longer overwritten on renewal. The link watchdog still runs a one-shot udhcpc if the default route disappears.',
  snmp: 'SNMP polling stops.',
  dropbear: 'The SSH daemon will not run until it is re-enabled.',
};

const DEFAULT_ENABLE_COPY =
  'The service starts immediately under the normal supervisor backoff policy.';

const ACTION_VERB: Record<ServiceAction, string> = {
  restart: 'Restart',
  enable: 'Enable',
  disable: 'Disable',
};

type PendingAction = { service: ServiceStatus; action: ServiceAction } | null;

function actionDescription(p: NonNullable<PendingAction>): string {
  if (p.action === 'restart') {
    return p.service.name === 'onvif'
      ? 'Restarting onvif also stops vendor-daemon — video and this page will drop with it and recover on their own when the camera returns.'
      : 'The supervisor sends SIGTERM; the service is restarted under its normal backoff policy.';
  }
  if (p.action === 'enable') {
    return DEFAULT_ENABLE_COPY;
  }
  return (
    DISABLE_COPY[p.service.name] ?? 'The service stops and will not run again until re-enabled.'
  );
}

function ServiceStateBadge({ service }: Readonly<{ service: ServiceStatus }>) {
  const state = service.state;
  return (
    <Badge
      className={
        state === 'running'
          ? 'border-transparent bg-green-500/10 text-green-500'
          : state === 'disabled'
            ? 'border-transparent bg-zinc-500/10 text-zinc-400'
            : 'border-transparent bg-amber-500/10 text-amber-500'
      }
      data-testid={`diagnostics-processes-status-${service.name}`}
    >
      {state}
    </Badge>
  );
}

export default function ProcessesCard() {
  const queryClient = useQueryClient();
  const [pending, setPending] = useState<PendingAction>(null);
  const [reconnecting, setReconnecting] = useState(false);
  const [onvifOff, setOnvifOff] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  const { data } = useQuery({
    queryKey: ['processes'],
    queryFn: ({ signal }) => getProcesses(signal),
    refetchInterval: REFRESH_MS,
  });

  useEffect(() => {
    return () => {
      abortRef.current?.abort();
    };
  }, []);

  const invalidate = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ['processes'] });
  }, [queryClient]);

  // The extracted restart path: unchanged behaviour, including the onvif
  // reconnecting state.
  const runRestart = useCallback(
    async (service: ServiceStatus) => {
      if (service.name !== 'onvif') {
        try {
          await restartService(service.name);
          toast.success(`Restart requested for ${service.name}`);
          invalidate();
        } catch (err) {
          toast.error(err instanceof Error ? err.message : 'Restart failed');
        }
        return;
      }

      // onvif: the POST's own connection is the one that gets dropped, so a
      // network-level failure here is the expected outcome, not an error.
      // An ApiError (404 unknown service / 409 disabled service / 503
      // supervisor unreachable) is a real failure — say so and do not wait for
      // a reboot that won't come.
      const controller = new AbortController();
      abortRef.current = controller;
      setReconnecting(true);
      try {
        try {
          await restartService(service.name);
        } catch (err) {
          if (err instanceof Error && isAbortError(err)) throw err;
          if (err instanceof Error && err.name === 'ApiError') {
            toast.error(err.message);
            return;
          }
          // fetch TypeError: connection dropped as the camera went down.
        }
        await waitForCameraBack((signal) => getDiagnostics(signal ?? controller.signal), {
          intervalMs: ONVIF_WAIT_INTERVAL_MS,
          timeoutMs: ONVIF_WAIT_TIMEOUT_MS,
          signal: controller.signal,
        });
      } catch (err) {
        if (!isAbortError(err)) {
          toast.error(err instanceof Error ? err.message : 'Lost the camera while waiting');
        }
      } finally {
        setReconnecting(false);
        invalidate();
      }
    },
    [invalidate],
  );

  const handleConfirm = useCallback(
    async (e: React.MouseEvent) => {
      e.preventDefault();
      const p = pending;
      if (!p) return;
      setPending(null);

      if (p.action === 'restart') {
        await runRestart(p.service);
        return;
      }

      // Disable/enable involves no reboot — except disabling onvif, which
      // takes down the very HTTP server serving this page.
      const isOnvifOff = p.action === 'disable' && p.service.name === 'onvif';
      const reportOnvifOff = () => {
        setOnvifOff(true);
        toast.success('onvif disabled — the camera is reachable via FTP only');
      };
      try {
        await serviceAction(p.service.name, p.action);
        if (isOnvifOff) reportOnvifOff();
        else toast.success(`${p.service.name} ${p.action === 'enable' ? 'enabled' : 'disabled'}`);
        invalidate();
      } catch (err) {
        if (err instanceof Error && err.name === 'ApiError') {
          toast.error(err.message);
        } else if (isOnvifOff) {
          // Network-level failure on an onvif disable: the only cause is the
          // camera killing our own connection — i.e. it worked.
          reportOnvifOff();
        } else {
          toast.error(err instanceof Error ? err.message : 'Toggle failed');
        }
      }
    },
    [invalidate, pending, runRestart],
  );

  const supervised = data?.supervised ?? null;

  return (
    <Card
      className="border-border bg-card overflow-hidden"
      data-testid="diagnostics-processes-card"
    >
      <CardHeader className="border-border border-b">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-blue-500/10">
            <Activity className="h-5 w-5 text-blue-500" />
          </div>
          <div>
            <CardTitle
              className="text-foreground text-sm font-semibold"
              data-testid="diagnostics-processes-title"
            >
              Processes
            </CardTitle>
            <p className="text-muted-foreground text-xs">
              Supervised services and running processes
            </p>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4 pt-4">
        {reconnecting && (
          <p
            className="text-muted-foreground text-sm"
            data-testid="diagnostics-processes-reconnecting"
            aria-live="polite"
          >
            onvif is restarting — the page will drop and recover on its own…
          </p>
        )}
        {onvifOff && (
          <p
            className="text-muted-foreground text-sm"
            data-testid="diagnostics-processes-onvif-off-note"
            aria-live="polite"
          >
            onvif is disabled — Web access is down until it is re-enabled via FTP or the SD card.
          </p>
        )}

        {data === undefined ? (
          <p className="text-muted-foreground text-sm" data-testid="diagnostics-processes-loading">
            Loading processes…
          </p>
        ) : (
          <>
            {supervised === null ? (
              <p
                className="text-muted-foreground text-sm"
                data-testid="diagnostics-processes-supervisor-note"
              >
                Supervisor control unavailable — showing all processes only.
              </p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-left text-sm">
                  <thead>
                    <tr className="text-muted-foreground border-border border-b text-xs">
                      <th className="py-2 pr-4 font-medium">Service</th>
                      <th className="py-2 pr-4 font-medium">State</th>
                      <th className="py-2 pr-4 font-medium">PID</th>
                      <th className="py-2 pr-4 font-medium">Uptime</th>
                      <th className="py-2 pr-4 font-medium">Restarts</th>
                      <th className="py-2 font-medium" />
                    </tr>
                  </thead>
                  <tbody>
                    {supervised.map((service) => {
                      const isDisabled = service.state === 'disabled';
                      const toggleable = !NON_TOGGLEABLE.has(service.name);
                      return (
                        <tr
                          key={service.name}
                          className={`border-border border-b last:border-b-0 ${isDisabled ? 'opacity-50' : ''}`}
                          data-testid={`diagnostics-processes-row-${service.name}`}
                        >
                          <td className="py-2 pr-4 font-mono">{service.name}</td>
                          <td className="py-2 pr-4">
                            <ServiceStateBadge service={service} />
                          </td>
                          <td
                            className="font-mono text-white"
                            data-testid={`diagnostics-processes-pid-${service.name}`}
                          >
                            {service.pid ?? '—'}
                          </td>
                          <td
                            className="font-mono text-white"
                            data-testid={`diagnostics-processes-uptime-${service.name}`}
                          >
                            {service.state === 'running' ? formatDuration(service.uptime_s) : '—'}
                            {service.state === 'backoff' && (
                              <span
                                className="text-muted-foreground"
                                data-testid={`diagnostics-processes-retry-countdown-${service.name}`}
                              >
                                {` restarts in ${service.retry_in_s}s`}
                              </span>
                            )}
                          </td>
                          <td className="font-mono text-white">{service.restarts}</td>
                          <td className="py-2">
                            <div className="flex justify-end gap-2">
                              {!isDisabled && (
                                <Button
                                  size="sm"
                                  variant="outline"
                                  data-testid={`diagnostics-processes-restart-${service.name}`}
                                  onClick={() => setPending({ service, action: 'restart' })}
                                >
                                  <RotateCw className="h-3.5 w-3.5" />
                                  Restart
                                </Button>
                              )}
                              {toggleable &&
                                (isDisabled ? (
                                  <Button
                                    size="sm"
                                    variant="outline"
                                    data-testid={`diagnostics-processes-enable-${service.name}`}
                                    onClick={() => setPending({ service, action: 'enable' })}
                                  >
                                    Enable
                                  </Button>
                                ) : (
                                  <Button
                                    size="sm"
                                    variant="outline"
                                    data-testid={`diagnostics-processes-disable-${service.name}`}
                                    onClick={() => setPending({ service, action: 'disable' })}
                                  >
                                    Disable
                                  </Button>
                                ))}
                            </div>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}

            <Collapsible>
              <CollapsibleTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-muted-foreground"
                  data-testid="diagnostics-processes-raw-trigger"
                >
                  <ChevronDown className="h-4 w-4" />
                  All processes ({data.processes.length})
                </Button>
              </CollapsibleTrigger>
              <CollapsibleContent>
                <div className="overflow-x-auto">
                  <table className="w-full text-left text-sm">
                    <thead>
                      <tr className="text-muted-foreground border-border border-b text-xs">
                        <th className="py-2 pr-4 font-medium">PID</th>
                        <th className="py-2 pr-4 font-medium">Name</th>
                        <th className="py-2 pr-4 font-medium">State</th>
                        <th className="py-2 pr-4 font-medium">RSS</th>
                        <th className="py-2 font-medium">CPU time</th>
                      </tr>
                    </thead>
                    <tbody>
                      {data.processes.map((proc) => (
                        <tr
                          key={proc.pid}
                          className="border-border border-b last:border-b-0"
                          data-testid={`diagnostics-processes-raw-row-${proc.pid}`}
                        >
                          <td className="py-2 pr-4 font-mono">{proc.pid}</td>
                          <td
                            className="py-2 pr-4 font-mono"
                            data-testid={`diagnostics-processes-raw-comm-${proc.pid}`}
                          >
                            {proc.comm}
                          </td>
                          <td className="py-2 pr-4 font-mono">{proc.state}</td>
                          <td
                            className="font-mono text-white"
                            data-testid={`diagnostics-processes-raw-rss-${proc.pid}`}
                          >
                            {Math.round(proc.rss_kb / 1024)} MB
                          </td>
                          <td
                            className="font-mono text-white"
                            data-testid={`diagnostics-processes-raw-cpu-${proc.pid}`}
                          >
                            {formatDuration(proc.cpu_time_s)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </CollapsibleContent>
            </Collapsible>
          </>
        )}
      </CardContent>

      <AlertDialog
        open={pending !== null}
        onOpenChange={(open) => {
          if (!open) setPending(null);
        }}
      >
        <AlertDialogContent
          className="bg-card border-border text-foreground"
          data-testid="diagnostics-processes-action-dialog"
        >
          <AlertDialogHeader>
            <AlertDialogTitle data-testid="diagnostics-processes-action-title">
              {pending ? `${ACTION_VERB[pending.action]} ${pending.service.name}?` : ''}
            </AlertDialogTitle>
            {pending && (
              <AlertDialogDescription data-testid="diagnostics-processes-action-description">
                {actionDescription(pending)}
              </AlertDialogDescription>
            )}
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel data-testid="diagnostics-processes-action-cancel">
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction
              data-testid="diagnostics-processes-action-confirm"
              onClick={(e) => {
                e.preventDefault();
                void handleConfirm(e);
              }}
            >
              {pending ? ACTION_VERB[pending.action] : ''}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Card>
  );
}
