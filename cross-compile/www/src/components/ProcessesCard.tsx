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
import { type ServiceStatus, getProcesses, restartService } from '@/services/processesService';
import { formatDuration } from '@/utils/formatDuration';

// One endpoint feeds both tables; split queries would double requests to save nothing.
const REFRESH_MS = 10_000;
// After a POST that drops the connection the camera needs a full reboot cycle.
const ONVIF_WAIT_INTERVAL_MS = 2000;
const ONVIF_WAIT_TIMEOUT_MS = 5 * 60 * 1000;

function ServiceStateBadge({ service }: Readonly<{ service: ServiceStatus }>) {
  const running = service.state === 'running';
  return (
    <Badge
      className={
        running
          ? 'border-transparent bg-green-500/10 text-green-500'
          : 'border-transparent bg-amber-500/10 text-amber-500'
      }
      data-testid={`diagnostics-processes-status-${service.name}`}
    >
      {service.state}
    </Badge>
  );
}

function RestartDescription({ service }: Readonly<{ service: ServiceStatus }>) {
  if (service.name !== 'onvif') {
    return (
      <AlertDialogDescription data-testid="diagnostics-processes-restart-dialog-description">
        The supervisor sends SIGTERM; the service is restarted under its normal backoff policy.
      </AlertDialogDescription>
    );
  }
  return (
    <AlertDialogDescription data-testid="diagnostics-processes-restart-dialog-description">
      Restarting onvif also stops vendor-daemon — video and this page will drop with it and recover
      on their own when the camera returns.
    </AlertDialogDescription>
  );
}

export default function ProcessesCard() {
  const queryClient = useQueryClient();
  const [target, setTarget] = useState<ServiceStatus | null>(null);
  const [reconnecting, setReconnecting] = useState(false);
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

  const handleConfirm = useCallback(
    async (e: React.MouseEvent) => {
      e.preventDefault();
      const service = target;
      if (!service) return;
      setTarget(null);

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
      // An ApiError (404 unknown service / 503 supervisor unreachable) is a
      // real failure — say so and do not wait for a reboot that won't come.
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
    [invalidate, target],
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
                    {supervised.map((service) => (
                      <tr
                        key={service.name}
                        className="border-border border-b last:border-b-0"
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
                        <td className="py-2 text-right">
                          <Button
                            size="sm"
                            variant="outline"
                            data-testid={`diagnostics-processes-restart-${service.name}`}
                            onClick={() => setTarget(service)}
                          >
                            <RotateCw className="h-3.5 w-3.5" />
                            Restart
                          </Button>
                        </td>
                      </tr>
                    ))}
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
        open={target !== null}
        onOpenChange={(open) => {
          if (!open) setTarget(null);
        }}
      >
        <AlertDialogContent
          className="bg-card border-border text-foreground"
          data-testid="diagnostics-processes-restart-dialog"
        >
          <AlertDialogHeader>
            <AlertDialogTitle data-testid="diagnostics-processes-restart-dialog-title">
              Restart {target?.name}?
            </AlertDialogTitle>
            {target && <RestartDescription service={target} />}
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel data-testid="diagnostics-processes-restart-cancel">
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction
              data-testid="diagnostics-processes-restart-confirm"
              onClick={(e) => {
                e.preventDefault();
                void handleConfirm(e);
              }}
            >
              Restart
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Card>
  );
}
