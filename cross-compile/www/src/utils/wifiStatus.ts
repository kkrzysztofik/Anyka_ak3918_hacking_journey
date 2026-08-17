import type { Diagnostics } from '@/services/diagnosticsService';

export type WifiDiagnostics = NonNullable<Diagnostics['wifi']>;

export function formatWifiChannel(wifi: WifiDiagnostics | null | undefined): string {
  if (wifi?.connected !== true) {
    return '—';
  }
  if (wifi.channel !== null && wifi.channel !== undefined) {
    return String(wifi.channel);
  }
  if (wifi.frequency_mhz !== null && wifi.frequency_mhz !== undefined) {
    return `${wifi.frequency_mhz} MHz`;
  }
  return '—';
}

export function formatWifiSecurity(wifi: WifiDiagnostics | null | undefined): string {
  if (wifi?.connected !== true) {
    return '—';
  }
  return wifi.security ?? '—';
}
