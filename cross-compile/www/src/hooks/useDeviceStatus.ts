import { useQuery } from '@tanstack/react-query';

import { getDiagnostics } from '@/services/diagnosticsService';
import { getNetworkInterfaces } from '@/services/networkService';
import {
  formatLinkSpeedMbps,
  pickPrimaryNetworkInterface,
  strictHealthStatus,
  systemUptimeLabel,
} from '@/utils/identificationStatusCard';
import { formatWifiChannel, formatWifiSecurity } from '@/utils/wifiStatus';

const STATUS_POLL_MS = 30_000;

/** Live device status for settings status cards (diagnostics + ONVIF network). */
export function useDeviceStatus() {
  const diagnosticsQuery = useQuery({
    queryKey: ['diagnostics'],
    queryFn: ({ signal }) => getDiagnostics(signal),
    refetchInterval: STATUS_POLL_MS,
  });

  const networkQuery = useQuery({
    queryKey: ['networkInterfaces'],
    queryFn: getNetworkInterfaces,
  });

  const primaryInterface = pickPrimaryNetworkInterface(networkQuery.data);
  const healthStatus = strictHealthStatus(diagnosticsQuery.data, {
    isError: diagnosticsQuery.isError,
    isLoading: diagnosticsQuery.isLoading,
  });

  return {
    diagnostics: diagnosticsQuery.data,
    primaryInterface,
    healthStatus,
    systemUptime: systemUptimeLabel(diagnosticsQuery.data),
    linkSpeed: formatLinkSpeedMbps(primaryInterface?.linkSpeedMbps),
    wifiChannel: formatWifiChannel(diagnosticsQuery.data?.wifi),
    wifiSecurity: formatWifiSecurity(diagnosticsQuery.data?.wifi),
    isLoading: diagnosticsQuery.isLoading || networkQuery.isLoading,
  };
}
