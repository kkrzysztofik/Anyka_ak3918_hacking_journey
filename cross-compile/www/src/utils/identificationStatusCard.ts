import type { Diagnostics } from '@/services/diagnosticsService';
import type { NetworkInterface } from '@/services/networkService';

import { formatDuration } from '@/utils/formatDuration';

export type HealthBadgeTone = 'healthy' | 'degraded' | 'unreachable' | 'unknown';

export interface StrictHealthStatus {
  label: string;
  tone: HealthBadgeTone;
  detail?: string;
}

const WLAN_PREFERENCE = ['wlan0', 'wlan1', 'eth0'];

/** Strict health from `/api/diagnostics` — never assume online from page load alone. */
export function strictHealthStatus(
  diagnostics: Diagnostics | undefined,
  options: { isError: boolean; isLoading: boolean },
): StrictHealthStatus {
  if (options.isLoading) {
    return { label: '—', tone: 'unknown' };
  }
  if (options.isError || diagnostics === undefined) {
    return { label: 'Unreachable', tone: 'unreachable' };
  }
  if (diagnostics.status === 'healthy' && diagnostics.degraded_services.length === 0) {
    return { label: 'Healthy', tone: 'healthy' };
  }
  if (diagnostics.status === 'degraded' || diagnostics.degraded_services.length > 0) {
    const detail =
      diagnostics.degraded_services.length > 0
        ? diagnostics.degraded_services.join(', ')
        : undefined;
    return { label: 'Degraded', tone: 'degraded', detail };
  }
  return {
    label: diagnostics.status,
    tone: 'degraded',
    detail:
      diagnostics.degraded_services.length > 0
        ? diagnostics.degraded_services.join(', ')
        : undefined,
  };
}

export function systemUptimeLabel(diagnostics: Diagnostics | undefined): string {
  if (diagnostics === undefined) {
    return '—';
  }
  return formatDuration(diagnostics.uptime.system_s);
}

/** Prefer Wi‑Fi when present; otherwise first enabled interface with a MAC. */
export function pickPrimaryNetworkInterface(
  interfaces: NetworkInterface[] | undefined,
): NetworkInterface | undefined {
  if (interfaces === undefined || interfaces.length === 0) {
    return undefined;
  }

  for (const preferred of WLAN_PREFERENCE) {
    const match = interfaces.find(
      (iface) => iface.name === preferred && iface.enabled && iface.hwAddress.length > 0,
    );
    if (match) {
      return match;
    }
  }

  return (
    interfaces.find((iface) => iface.enabled && iface.hwAddress.length > 0) ?? interfaces[0]
  );
}

export function formatLinkSpeedMbps(speedMbps: number | null | undefined): string {
  if (speedMbps === null || speedMbps === undefined || speedMbps <= 0) {
    return '—';
  }
  return `${speedMbps} Mbps`;
}
