/**
 * Diagnostics Service
 *
 * JSON GET operations for the /api/diagnostics and /api/logs endpoints.
 * Uses authorizedFetch so 401 responses trigger the shared session-expiry path.
 */
import {
  ApiError,
  authorizedFetch,
  authorizedXhrPut,
  type UploadProgress,
} from '@/services/api';

export interface Diagnostics {
  status: string;
  /** `git describe` at build time on the running bundle. */
  firmware_version: string;
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
  ptz?: {
    enabled: boolean;
    opened: boolean;
    init_error: string | null;
    self_check: string | null;
    /** Tracked pan/tilt/zoom in degrees — dead-reckoned, not measured. */
    position: [number, number, number] | null;
    moving: boolean;
    last_step_pos: { pan: number | null; tilt: number | null; age_ms: number } | null;
    /** Whether last_step_pos is a measurement. Absent on pre-2026-08-16 bundles. */
    step_readback?: 'working' | 'unsupported' | 'unknown';
    commands_completed: number;
  } | null;
  wifi?: {
    interface: string;
    connected: boolean;
    ssid: string | null;
    frequency_mhz: number | null;
    channel: number | null;
    security: string | null;
    signal_dbm: number | null;
    link_quality: string | null;
  } | null;
}

export type LogSource = 'onvif_rust' | 'vendor_daemon' | 'anyka_init' | 'wpa_supplicant';
export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isVisionSupported(
  value: unknown,
): value is NonNullable<Diagnostics['vision']>['supported'] {
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

function isPositionTuple(value: unknown): value is [number, number, number] {
  return (
    Array.isArray(value) &&
    value.length === 3 &&
    typeof value[0] === 'number' &&
    typeof value[1] === 'number' &&
    typeof value[2] === 'number'
  );
}

function isLastStepPos(
  value: unknown,
): value is NonNullable<NonNullable<Diagnostics['ptz']>['last_step_pos']> {
  return (
    isRecord(value) &&
    isNullOrNumber(value.pan) &&
    isNullOrNumber(value.tilt) &&
    typeof value.age_ms === 'number'
  );
}

function isPtz(value: unknown): value is NonNullable<Diagnostics['ptz']> {
  return (
    isRecord(value) &&
    typeof value.enabled === 'boolean' &&
    typeof value.opened === 'boolean' &&
    typeof value.moving === 'boolean' &&
    typeof value.commands_completed === 'number' &&
    (value.init_error === null || typeof value.init_error === 'string') &&
    (value.self_check === null || typeof value.self_check === 'string') &&
    (value.position === null || isPositionTuple(value.position)) &&
    (value.last_step_pos === null || isLastStepPos(value.last_step_pos)) &&
    (value.step_readback === undefined ||
      value.step_readback === 'working' ||
      value.step_readback === 'unsupported' ||
      value.step_readback === 'unknown')
  );
}

function isNullOrNumber(value: unknown): boolean {
  return value === null || typeof value === 'number';
}

function isNullOrString(value: unknown): boolean {
  return value === null || typeof value === 'string';
}

function isWifi(value: unknown): value is NonNullable<Diagnostics['wifi']> {
  return (
    isRecord(value) &&
    typeof value.interface === 'string' &&
    typeof value.connected === 'boolean' &&
    isNullOrString(value.ssid) &&
    isNullOrNumber(value.frequency_mhz) &&
    isNullOrNumber(value.channel) &&
    isNullOrString(value.security) &&
    (value.signal_dbm === null || typeof value.signal_dbm === 'number') &&
    isNullOrString(value.link_quality)
  );
}

function isNullOrRecord(value: unknown): value is Record<string, unknown> | null {
  return value === null || isRecord(value);
}

function isUptime(value: unknown): value is Diagnostics['uptime'] {
  return (
    isRecord(value) && typeof value.process_s === 'number' && typeof value.system_s === 'number'
  );
}

function isDiagnostics(value: unknown): value is Diagnostics {
  if (!isRecord(value)) return false;
  if (typeof value.status !== 'string') return false;
  if (typeof value.firmware_version !== 'string') return false;
  if (!isUptime(value.uptime)) return false;
  if (!isNullOrNumber(value.cpu_percent)) return false;
  if (!isNullOrRecord(value.memory)) return false;
  if (!isNullOrRecord(value.storage)) return false;
  if (!isNullOrRecord(value.network)) return false;
  if (!isNullOrNumber(value.stream_frame_age_ms)) return false;
  if (!Array.isArray(value.components) || !Array.isArray(value.degraded_services)) return false;
  if (value.vision !== null && !isVision(value.vision)) return false;
  // Absent is fine — a snapshot from a build without PTZ reporting still validates.
  if (value.ptz !== null && value.ptz !== undefined && !isPtz(value.ptz)) return false;
  if (value.wifi !== null && value.wifi !== undefined && !isWifi(value.wifi)) return false;
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
    throw new ApiError(`Logs request failed with status ${response.status}`, response.status, text);
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

/**
 * Upload an upgrade bundle to the camera.
 *
 * Streams the file raw as the PUT body — the backend never buffers it, and
 * neither should we. 202 means the update is queued: the applier will stage it,
 * flip slots and reboot within the next poll interval.
 */
export async function uploadFirmware(
  file: File,
  options?: {
    onProgress?: (p: UploadProgress) => void;
    signal?: AbortSignal;
  },
): Promise<void> {
  const { status, bodyText } = await authorizedXhrPut('/api/update', file, options);

  if (status === 202) {
    return;
  }
  throw new ApiError(`Firmware upload failed with status ${status}`, status, bodyText);
}
