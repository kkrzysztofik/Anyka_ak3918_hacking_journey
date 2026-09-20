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

/**
 * The live association's security as the Network form's Security select spells
 * it. Falls back to `wpa` when nothing is associated — the overwhelmingly
 * common case, and the only safe default to save back.
 */
export function wifiSecurityMode(wifi: WifiDiagnostics | null | undefined): 'wpa' | 'wep' | 'open' {
  if (wifi?.connected !== true) {
    return 'wpa';
  }
  switch (wifi.security) {
    case 'Open':
      return 'open';
    case 'WEP':
      return 'wep';
    default:
      return 'wpa';
  }
}

export function formatWifiQuality(wifi: WifiDiagnostics | null | undefined): string {
  if (wifi?.connected !== true) {
    return '—';
  }
  return wifi.link_quality ?? '—';
}
