/**
 * Diagnostics Service
 *
 * JSON GET operations for the /api/diagnostics and /api/logs endpoints.
 * Uses authorizedFetch so 401 responses trigger the shared session-expiry path.
 */
import { ApiError, authorizedFetch } from '@/services/api';

export interface Diagnostics {
  status: string;
  uptime: { process_s: number; system_s: number };
  cpu_percent: number | null;
  memory: { total_kb: number; used_kb: number } | null;
  storage: { total_kb: number; used_kb: number } | null;
  network: { rx_bps: number; tx_bps: number } | null;
  stream_frame_age_ms: number | null;
  components: Array<{ name: string; status: string; message: string | null }>;
  degraded_services: string[];
  vision: {
    mode: string | null;
    ae_luma: number | null;
    ain0: number | null;
    ir_led: boolean | null;
    ircut_a: boolean | null;
    ircut_b: boolean | null;
    white_led: boolean | null;
    supported: { ir_led: boolean; ircut: boolean; white_led: boolean };
  } | null;
}

export type LogSource = 'onvif_rust' | 'vendor_daemon' | 'anyka_init' | 'wpa_supplicant';
export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isVisionSupported(value: unknown): value is NonNullable<Diagnostics['vision']>['supported'] {
  return (
    isRecord(value) &&
    typeof value.ir_led === 'boolean' &&
    typeof value.ircut === 'boolean' &&
    typeof value.white_led === 'boolean'
  );
}

function isVision(value: unknown): value is NonNullable<Diagnostics['vision']> {
  return (
    isRecord(value) &&
    (value.mode === null || typeof value.mode === 'string') &&
    (value.ae_luma === null || typeof value.ae_luma === 'number') &&
    (value.ain0 === null || typeof value.ain0 === 'number') &&
    (value.ir_led === null || typeof value.ir_led === 'boolean') &&
    (value.ircut_a === null || typeof value.ircut_a === 'boolean') &&
    (value.ircut_b === null || typeof value.ircut_b === 'boolean') &&
    (value.white_led === null || typeof value.white_led === 'boolean') &&
    isVisionSupported(value.supported)
  );
}

function isNullOrNumber(value: unknown): boolean {
  return value === null || typeof value === 'number';
}

function isNullOrRecord(value: unknown): value is Record<string, unknown> | null {
  return value === null || isRecord(value);
}

function isUptime(value: unknown): value is Diagnostics['uptime'] {
  return (
    isRecord(value) &&
    typeof value.process_s === 'number' &&
    typeof value.system_s === 'number'
  );
}

function isDiagnostics(value: unknown): value is Diagnostics {
  if (!isRecord(value)) return false;
  if (typeof value.status !== 'string') return false;
  if (!isUptime(value.uptime)) return false;
  if (!isNullOrNumber(value.cpu_percent)) return false;
  if (!isNullOrRecord(value.memory)) return false;
  if (!isNullOrRecord(value.storage)) return false;
  if (!isNullOrRecord(value.network)) return false;
  if (!isNullOrNumber(value.stream_frame_age_ms)) return false;
  if (!Array.isArray(value.components) || !Array.isArray(value.degraded_services)) return false;
  if (value.vision !== null && !isVision(value.vision)) return false;
  return true;
}

function isLogLines(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((line) => typeof line === 'string');
}

/**
 * Fetch a current diagnostics snapshot from the backend.
 */
export async function getDiagnostics(signal?: AbortSignal): Promise<Diagnostics> {
  const response = await authorizedFetch('/api/diagnostics', { method: 'GET', signal });

  if (!response.ok) {
    const text = await response.text();
    throw new ApiError(
      `Diagnostics request failed with status ${response.status}`,
      response.status,
      text,
    );
  }

  let payload: unknown;
  try {
    payload = await response.json();
  } catch {
    throw new ApiError('Invalid JSON in diagnostics response', response.status, '');
  }
  if (!isDiagnostics(payload)) {
    throw new ApiError('Diagnostics response has an unexpected shape', response.status, '');
  }
  return payload;
}

/**
 * Fetch recent log lines for a given source.
 *
 * Returns an empty array when the source is not found (404) — this is normal
 * when a component hasn't started yet or was never configured.
 */
export async function getLogs(
  source: LogSource,
  level?: LogLevel,
  lines = 200,
  signal?: AbortSignal,
): Promise<string[]> {
  const params = new URLSearchParams({ source, lines: String(lines) });
  if (level !== undefined) {
    params.set('level', level);
  }

  const response = await authorizedFetch(`/api/logs?${params.toString()}`, {
    method: 'GET',
    signal,
  });

  if (response.status === 404) {
    return [];
  }

  if (!response.ok) {
    const text = await response.text();
    throw new ApiError(
      `Logs request failed with status ${response.status}`,
      response.status,
      text,
    );
  }

  let payload: unknown;
  try {
    payload = await response.json();
  } catch {
    throw new ApiError('Invalid JSON in logs response', response.status, '');
  }
  if (!isLogLines(payload)) {
    throw new ApiError('Logs response has an unexpected shape', response.status, '');
  }
  return payload;
}
