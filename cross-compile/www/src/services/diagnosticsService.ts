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

  return response.json() as Promise<Diagnostics>;
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

  return response.json() as Promise<string[]>;
}
