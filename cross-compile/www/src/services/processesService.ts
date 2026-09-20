/**
 * Processes Service
 *
 * JSON operations for the /api/processes and the /api/services/{name}/
 * restart|enable|disable endpoints. Uses authorizedFetch so 401 responses
 * trigger the shared session-expiry path, and validates runtime shapes by
 * hand like the other services — no schema library.
 */
import { ApiError, authorizedFetch } from '@/services/api';

export interface Process {
  pid: number;
  ppid: number;
  comm: string;
  /** Single-letter state from /proc/[pid]/stat: R, S, D, Z, T. */
  state: string;
  rss_kb: number;
  /** Cumulative utime+stime in seconds. */
  cpu_time_s: number;
}

export interface ServiceStatus {
  name: string;
  /** 'running' | 'backoff' | 'disabled'. */
  state: string;
  /** Null when the service is not running. */
  pid: number | null;
  uptime_s: number;
  restarts: number;
  retry_in_s: number;
}

export interface ProcessesResponse {
  /** Null when the anyka-init control socket is unreachable. */
  supervised: ServiceStatus[] | null;
  processes: Process[];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isNumber(value: unknown): value is number {
  return typeof value === 'number';
}

function isServiceStatus(value: unknown): value is ServiceStatus {
  return (
    isRecord(value) &&
    typeof value.name === 'string' &&
    typeof value.state === 'string' &&
    (value.pid === null || isNumber(value.pid)) &&
    isNumber(value.uptime_s) &&
    isNumber(value.restarts) &&
    isNumber(value.retry_in_s)
  );
}

function isProcess(value: unknown): value is Process {
  return (
    isRecord(value) &&
    isNumber(value.pid) &&
    isNumber(value.ppid) &&
    typeof value.comm === 'string' &&
    typeof value.state === 'string' &&
    isNumber(value.rss_kb) &&
    isNumber(value.cpu_time_s)
  );
}

function isProcessesResponse(value: unknown): value is ProcessesResponse {
  if (!isRecord(value)) return false;
  if (value.supervised !== null) {
    if (!Array.isArray(value.supervised) || !value.supervised.every(isServiceStatus)) {
      return false;
    }
  }
  return Array.isArray(value.processes) && value.processes.every(isProcess);
}

/**
 * Fetch the supervised-service snapshot plus the raw process table.
 *
 * `supervised` is null when the anyka-init control socket is unreachable
 * (older binary in the other A/B slot, or the control thread failed to bind)
 * — that is a degraded state, not an error.
 */
export async function getProcesses(signal?: AbortSignal): Promise<ProcessesResponse> {
  const response = await authorizedFetch('/api/processes', { method: 'GET', signal });

  if (!response.ok) {
    const text = await response.text();
    throw new ApiError(
      `Processes request failed with status ${response.status}`,
      response.status,
      text,
    );
  }

  let payload: unknown;
  try {
    payload = await response.json();
  } catch {
    throw new ApiError('Invalid JSON in processes response', response.status, '');
  }
  if (!isProcessesResponse(payload)) {
    throw new ApiError('Processes response has an unexpected shape', response.status, '');
  }
  return payload;
}

export type ServiceAction = 'restart' | 'enable' | 'disable';

/**
 * Restart, enable or disable a supervised service.
 *
 * 202 means the supervisor accepted: for a toggle the on-disk config is
 * already written, and state transitions on the supervisor's schedule. A 404
 * (unknown or non-toggleable service), 409 (restart of a disabled service) or
 * 503 (supervisor unreachable / config write failed) throws an ApiError. A
 * network-level failure — the camera dropping the connection as onvif itself
 * goes down — surfaces as a fetch TypeError, so callers can distinguish "it
 * did this to us" from "it failed".
 */
export async function serviceAction(name: string, action: ServiceAction): Promise<void> {
  const response = await authorizedFetch(`/api/services/${encodeURIComponent(name)}/${action}`, {
    method: 'POST',
  });

  if (response.status === 202) {
    return;
  }
  const text = await response.text();
  throw new ApiError(
    `${action} of ${name} failed with status ${response.status}`,
    response.status,
    text,
  );
}
