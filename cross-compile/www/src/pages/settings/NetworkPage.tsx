/**
 * Network Page
 *
 * Configure network interfaces, DNS, Wi-Fi, and ports.
 */
import React, { useCallback, useEffect, useMemo, useState } from 'react';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Globe, Info, Network, RotateCcw, Save, Server, Wifi, X } from 'lucide-react';
import { Resolver, useForm, useWatch } from 'react-hook-form';
import { toast } from 'sonner';
import { z } from 'zod';

import { HealthStatusValue } from '@/components/settings/HealthStatusValue';
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
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  SettingsCard,
  SettingsCardContent,
  SettingsCardDescription,
  SettingsCardHeader,
  SettingsCardTitle,
} from '@/components/ui/settings-card';
import {
  StatusCard,
  StatusCardContent,
  StatusCardImage,
  StatusCardItem,
} from '@/components/ui/status-card';
import { Switch } from '@/components/ui/switch';
import { useDeviceStatus } from '@/hooks/useDeviceStatus';
import {
  type NetworkConfig,
  getNetworkConfig,
  getNetworkOverlay,
  getSnmpConfig,
  putNetworkOverlay,
  putSnmpConfig,
  setDNS,
  setNetworkDefaultGateway,
  setNetworkInterface,
  setNetworkProtocols,
} from '@/services/networkService';

const octet = String.raw`(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)`;
const ipRegex = new RegExp(String.raw`^${octet}\.${octet}\.${octet}\.${octet}$`);

const networkSchema = z
  .object({
    ssid: z.string().max(32, 'SSID must be 32 characters or fewer'),
    password: z.string().max(63, 'WPA passphrases are at most 63 characters'),
    security: z.enum(['wpa', 'wep', 'open']),
    dhcp: z.boolean(),
    address: z.string().regex(ipRegex, 'Invalid IP address').optional().or(z.literal('')),
    prefixLength: z.number().int().min(1).max(32).optional(),
    gateway: z.string().regex(ipRegex, 'Invalid IP address').optional().or(z.literal('')),
    dnsFromDHCP: z.boolean(),
    primaryDNS: z.string().regex(ipRegex, 'Invalid IP address').optional().or(z.literal('')),
    secondaryDNS: z.string().regex(ipRegex, 'Invalid IP address').optional().or(z.literal('')),
    httpPort: z.number().int().min(1, 'Port must be 1-65535').max(65535, 'Port must be 1-65535'),
    rtspPort: z.number().int().min(1, 'Port must be 1-65535').max(65535, 'Port must be 1-65535'),
    snmpEnabled: z.boolean(),
    snmpPort: z.number().int().min(1, 'Port must be 1-65535').max(65535, 'Port must be 1-65535'),
    snmpCommunity: z.string().max(64),
  })
  .superRefine((data, ctx) => {
    if (!data.dhcp) {
      if (!data.address?.trim()) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['address'],
          message: 'IP address is required when DHCP is disabled',
        });
      }
      if (!data.gateway?.trim()) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['gateway'],
          message: 'Gateway is required when DHCP is disabled',
        });
      }
    }
    if (data.snmpEnabled && !data.snmpCommunity.trim()) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['snmpCommunity'],
        message: 'Community must not be empty',
      });
    }
  });

type NetworkFormData = z.infer<typeof networkSchema>;

function parseOverlayAddress(address?: string): { ip: string; prefix: number } | null {
  if (!address) return null;
  const [ip, prefixStr] = address.split('/');
  if (!ip || !prefixStr) return null;
  const prefix = Number(prefixStr);
  if (!Number.isInteger(prefix) || prefix < 1 || prefix > 32) return null;
  return { ip, prefix };
}

function ipOverlayDiffersFromLive(
  config: NetworkConfig | undefined,
  overlay: Awaited<ReturnType<typeof getNetworkOverlay>> | undefined,
): boolean {
  if (!config || !overlay?.has_pending) return false;
  const pending = overlay.pending;
  const iface = config.interfaces[0];
  if (!iface) return false;

  if (pending.dhcp !== undefined && pending.dhcp !== iface.dhcp) return true;
  const parsed = parseOverlayAddress(pending.address);
  if (parsed && (parsed.ip !== iface.address || parsed.prefix !== iface.prefixLength)) {
    return true;
  }
  if (pending.gateway && pending.gateway !== iface.gateway) return true;
  if (pending.dns && pending.dns.join(',') !== config.dns.dnsServers.join(',')) return true;
  return false;
}

function buildWifiOverlayPatch(
  values: NetworkFormData,
  liveSsid: string | undefined,
  liveSecurity: NetworkFormData['security'],
): Parameters<typeof putNetworkOverlay>[0] | null {
  const patch: Parameters<typeof putNetworkOverlay>[0] = {};
  if (values.ssid && values.ssid !== liveSsid) {
    patch.ssid = values.ssid;
  }
  if (values.security !== liveSecurity) {
    patch.security = values.security;
  }
  if (values.password) {
    patch.password = values.password;
  }
  return Object.keys(patch).length > 0 ? patch : null;
}

async function runNetworkStep(label: string, step: () => Promise<void>): Promise<void> {
  try {
    await step();
  } catch (error) {
    const message = error instanceof Error ? error.message : 'unknown error';
    throw new Error(`${label}: ${message}`, { cause: error });
  }
}

export default function NetworkPage() {
  const queryClient = useQueryClient();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [pendingValues, setPendingValues] = useState<NetworkFormData | null>(null);
  const [snmpOnlySave, setSnmpOnlySave] = useState(false);
  const [failureDismissed, setFailureDismissed] = useState(false);
  const dismissFailure = useCallback(() => setFailureDismissed(true), []);

  const { data: config, isLoading } = useQuery<NetworkConfig>({
    queryKey: ['networkConfig'],
    queryFn: getNetworkConfig,
  });

  const { data: overlay } = useQuery({
    queryKey: ['networkOverlay'],
    queryFn: getNetworkOverlay,
  });

  const {
    data: snmp,
    isSuccess: snmpLoaded,
    isError: snmpError,
    isPending: snmpPending,
    error: snmpLoadError,
  } = useQuery({
    queryKey: ['snmpConfig'],
    queryFn: getSnmpConfig,
  });

  const form = useForm<NetworkFormData>({
    resolver: zodResolver(networkSchema) as Resolver<NetworkFormData>,
    defaultValues: {
      ssid: '',
      password: '',
      security: 'wpa',
      dhcp: true,
      address: '',
      prefixLength: 24,
      gateway: '',
      dnsFromDHCP: true,
      primaryDNS: '',
      secondaryDNS: '',
      httpPort: 80,
      rtspPort: 554,
      snmpEnabled: true,
      snmpPort: 161,
      snmpCommunity: 'public',
    },
  });

  const dhcpEnabled = useWatch({ control: form.control, name: 'dhcp' });
  const dnsFromDHCP = useWatch({ control: form.control, name: 'dnsFromDHCP' });
  const httpPort = useWatch({ control: form.control, name: 'httpPort' });
  const { healthStatus, primaryInterface, systemUptime, wifiQuality, diagnostics } =
    useDeviceStatus();

  const ipPending = useMemo(() => ipOverlayDiffersFromLive(config, overlay), [config, overlay]);
  const snmpUnavailable = snmpPending || snmpError;

  useEffect(() => {
    if (!config) return;
    const iface = config.interfaces[0];
    const pending = overlay?.pending;
    const parsed = parseOverlayAddress(pending?.address);
    const snmpFields = {
      snmpEnabled: form.getValues('snmpEnabled'),
      snmpPort: form.getValues('snmpPort'),
      snmpCommunity: form.getValues('snmpCommunity'),
    };
    form.reset({
      ssid: pending?.ssid ?? diagnostics?.wifi?.ssid ?? '',
      password: '',
      security: (pending?.security as NetworkFormData['security']) ?? 'wpa',
      dhcp: pending?.dhcp ?? iface?.dhcp ?? true,
      address: parsed?.ip ?? iface?.address ?? '',
      prefixLength: parsed?.prefix ?? iface?.prefixLength ?? 24,
      gateway: pending?.gateway ?? iface?.gateway ?? '',
      dnsFromDHCP: config.dns.fromDHCP,
      primaryDNS: pending?.dns?.[0] ?? config.dns.dnsServers[0] ?? '',
      secondaryDNS: pending?.dns?.[1] ?? config.dns.dnsServers[1] ?? '',
      httpPort: config.protocols.http,
      rtspPort: config.protocols.rtsp,
      ...snmpFields,
    });
  }, [config, overlay, diagnostics?.wifi?.ssid, form]);

  useEffect(() => {
    if (!snmpLoaded || !snmp || form.formState.isDirty) return;
    form.setValue('snmpEnabled', snmp.enabled);
    form.setValue('snmpPort', snmp.port);
    form.setValue('snmpCommunity', snmp.community);
  }, [snmp, snmpLoaded, form, form.formState.isDirty]);

  const mutation = useMutation({
    mutationFn: async (values: NetworkFormData) => {
      const iface = config?.interfaces[0];
      if (!iface) throw new Error('No interface found');

      // diagnostics reports ssid: null when wlan0 is not associated; the patch
      // builder takes string | undefined, so collapse null into undefined.
      const liveSsid = overlay?.pending?.ssid ?? diagnostics?.wifi?.ssid ?? undefined;
      const liveSecurity =
        (overlay?.pending?.security as NetworkFormData['security'] | undefined) ?? 'wpa';
      const wifiPatch = buildWifiOverlayPatch(values, liveSsid, liveSecurity);
      if (wifiPatch) {
        await runNetworkStep('Wi-Fi configuration failed', () => putNetworkOverlay(wifiPatch));
      }

      await runNetworkStep('IP configuration failed', () =>
        setNetworkInterface(iface.token, values.dhcp, values.address, values.prefixLength),
      );

      const gateway = values.gateway;
      if (!values.dhcp && gateway) {
        await runNetworkStep('Gateway failed', () => setNetworkDefaultGateway(gateway));
      }

      const dnsServers = [values.primaryDNS, values.secondaryDNS].filter(Boolean) as string[];
      await runNetworkStep('DNS failed', () => setDNS(values.dnsFromDHCP, dnsServers));
      await runNetworkStep('Port configuration failed', () =>
        setNetworkProtocols(values.httpPort, values.rtspPort),
      );
      const snmpChanged =
        snmp !== undefined &&
        (values.snmpEnabled !== snmp.enabled ||
          values.snmpPort !== snmp.port ||
          values.snmpCommunity !== snmp.community);
      if (snmpChanged) {
        if (!snmpLoaded) {
          throw new Error('SNMP configuration is still loading');
        }
        if (snmpError) {
          throw new Error(
            snmpLoadError == null ? 'SNMP configuration failed to load' : String(snmpLoadError),
          );
        }
        await runNetworkStep('SNMP configuration failed', () =>
          putSnmpConfig({
            enabled: values.snmpEnabled,
            port: values.snmpPort,
            community: values.snmpCommunity,
          }),
        );
      }
    },
    onSuccess: () => {
      toast.success('Network settings saved', {
        description:
          'Changes are saved and will apply after the next reboot. The device may be unreachable until then if IP or ports changed. SNMP changes apply on reload without reboot.',
      });
      queryClient.invalidateQueries({ queryKey: ['networkConfig'] });
      queryClient.invalidateQueries({ queryKey: ['networkOverlay'] });
      queryClient.invalidateQueries({ queryKey: ['snmpConfig'] });
      setConfirmOpen(false);
    },
    onError: (error) => {
      toast.error('Failed to save settings', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
      setConfirmOpen(false);
    },
  });

  const onSubmit = (values: NetworkFormData) => {
    const dirtyKeys = Object.keys(form.formState.dirtyFields);
    setSnmpOnlySave(dirtyKeys.length > 0 && dirtyKeys.every((key) => key.startsWith('snmp')));
    setPendingValues(values);
    setConfirmOpen(true);
  };

  const handleConfirm = () => {
    if (pendingValues) {
      mutation.mutate(pendingValues);
    }
  };

  const handleReset = () => {
    if (config) {
      const iface = config.interfaces[0];
      const pending = overlay?.pending;
      const parsed = parseOverlayAddress(pending?.address);
      form.reset({
        ssid: pending?.ssid ?? diagnostics?.wifi?.ssid ?? '',
        password: '',
        security: (pending?.security as NetworkFormData['security']) ?? 'wpa',
        dhcp: pending?.dhcp ?? iface?.dhcp ?? true,
        address: parsed?.ip ?? iface?.address ?? '',
        prefixLength: parsed?.prefix ?? iface?.prefixLength ?? 24,
        gateway: pending?.gateway ?? iface?.gateway ?? '',
        dnsFromDHCP: config.dns.fromDHCP,
        primaryDNS: pending?.dns?.[0] ?? config.dns.dnsServers[0] ?? '',
        secondaryDNS: pending?.dns?.[1] ?? config.dns.dnsServers[1] ?? '',
        httpPort: config.protocols.http,
        rtspPort: config.protocols.rtsp,
        snmpEnabled: snmp?.enabled ?? false,
        snmpPort: snmp?.port ?? 161,
        snmpCommunity: snmp?.community ?? '',
      });
      toast.info('Form reset to current values');
    }
  };

  const confirmDescription = useMemo(() => {
    if (snmpOnlySave) {
      return 'SNMP changes apply on reload without reboot.';
    }
    const base =
      'Applying these changes might disconnect the device from the network. Settings take effect after reboot.';
    if (pendingValues && pendingValues.httpPort !== config?.protocols.http) {
      return `${base} After reboot, open the WebUI at http://${globalThis.location.hostname}:${pendingValues.httpPort}/`;
    }
    return base;
  }, [snmpOnlySave, pendingValues, config?.protocols.http]);

  if (isLoading)
    return (
      <div className="text-white" data-testid="network-loading">
        Loading...
      </div>
    );

  return (
    <div
      className="absolute inset-0 overflow-auto bg-[#0d0d0d] lg:inset-[0_0_0_356.84px]"
      data-name="Container"
    >
      <div className="max-w-[1200px] p-[16px] pb-[80px] md:p-[32px] md:pb-[48px] lg:p-[48px]">
        <div className="mb-[32px] md:mb-[40px]">
          <h1
            className="mb-[8px] text-[22px] text-white md:text-[28px]"
            data-testid="network-title"
          >
            Network
          </h1>
          <p className="text-[13px] text-[#a1a1a6] md:text-[14px]">
            Configure IP address, DNS, Wi-Fi, and service ports
          </p>
          <p className="mt-[8px] text-[13px] text-[#a1a1a6]">
            Hostname and ONVIF discovery are configured under{' '}
            <a
              href="#/settings/identification"
              className="text-[#0a84ff] underline"
              data-testid="network-identification-link"
            >
              Settings › Identification
            </a>
            {'.'}
          </p>
        </div>

        {overlay?.last_failure && !failureDismissed && (
          <div
            role="alert"
            className="mb-[24px] flex items-start justify-between gap-[12px] rounded-[8px] border border-[rgba(255,69,58,0.3)] bg-[rgba(255,69,58,0.08)] p-[16px]"
            data-testid="network-failure-banner"
          >
            <p className="text-[13px] text-[#ff453a]">
              Previous network settings failed and were reverted
              {overlay.last_failure.ssid ? ` (SSID: ${overlay.last_failure.ssid})` : ''}. Review the
              values below before saving again.
            </p>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="shrink-0 text-[#ff453a]"
              aria-label="Dismiss network failure"
              onClick={dismissFailure}
              data-testid="network-failure-dismiss"
            >
              <X className="size-4" />
            </Button>
          </div>
        )}

        <StatusCard>
          <StatusCardImage>
            <Network className="size-8 opacity-50" />
          </StatusCardImage>
          <StatusCardContent>
            <StatusCardItem
              label="MAC Address"
              value={primaryInterface?.hwAddress || '—'}
              data-testid="network-mac-address"
            />
            <StatusCardItem
              label="Link Quality"
              value={wifiQuality}
              data-testid="network-quality"
            />
            <StatusCardItem
              label="Status"
              value={
                <HealthStatusValue
                  label={healthStatus.label}
                  tone={healthStatus.tone}
                  detail={healthStatus.detail}
                  testId="network-status"
                />
              }
            />
            <StatusCardItem label="Uptime" value={systemUptime} data-testid="network-uptime" />
          </StatusCardContent>
        </StatusCard>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-[24px]">
            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(255,159,10,0.1)]">
                    <Wifi className="size-5 text-[#ff9f0a]" />
                  </div>
                  <div>
                    <SettingsCardTitle>Wi-Fi Network</SettingsCardTitle>
                    <SettingsCardDescription>
                      Credentials applied at the next reboot
                    </SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="space-y-[24px]">
                <FormField
                  control={form.control}
                  name="ssid"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[#a1a1a6]">SSID</FormLabel>
                      <FormControl>
                        <Input
                          {...field}
                          className="border-[#3a3a3c] bg-transparent text-white focus:border-[#0a84ff]"
                          data-testid="network-ssid-input"
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="password"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[#a1a1a6]">Password</FormLabel>
                      <FormControl>
                        <Input
                          {...field}
                          type="password"
                          placeholder={
                            overlay?.pending.has_password ? 'Saved (leave blank to keep)' : ''
                          }
                          className="border-[#3a3a3c] bg-transparent text-white focus:border-[#0a84ff]"
                          data-testid="network-password-input"
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="security"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[#a1a1a6]">Security</FormLabel>
                      <Select onValueChange={field.onChange} value={field.value}>
                        <FormControl>
                          <SelectTrigger
                            className="border-[#3a3a3c] bg-transparent text-white"
                            data-testid="network-security-select"
                          >
                            <SelectValue placeholder="Select security" />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          <SelectItem value="wpa">WPA/WPA2</SelectItem>
                          <SelectItem value="wep">WEP</SelectItem>
                          <SelectItem value="open">Open</SelectItem>
                        </SelectContent>
                      </Select>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </SettingsCardContent>
            </SettingsCard>

            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(10,132,255,0.1)]">
                    <Globe className="size-5 text-[#0a84ff]" />
                  </div>
                  <div className="flex flex-1 items-center justify-between gap-[12px]">
                    <div>
                      <SettingsCardTitle>IP Configuration</SettingsCardTitle>
                      <SettingsCardDescription>
                        Addressing applied at next reboot
                      </SettingsCardDescription>
                    </div>
                    {ipPending && (
                      <Badge
                        variant="outline"
                        className="border-[#ff9f0a] text-[#ff9f0a]"
                        data-testid="network-ip-pending-badge"
                      >
                        Pending reboot
                      </Badge>
                    )}
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="space-y-[24px]">
                <FormField
                  control={form.control}
                  name="dhcp"
                  render={({ field }) => (
                    <FormItem className="flex flex-row items-center justify-between rounded-lg border border-[#3a3a3c] bg-[#2c2c2e] p-4">
                      <div>
                        <FormLabel className="text-base leading-none text-white">DHCP</FormLabel>
                        <FormDescription className="text-[#a1a1a6]">
                          Automatically obtain IP settings from the router
                        </FormDescription>
                      </div>
                      <FormControl>
                        <Switch
                          checked={field.value}
                          onCheckedChange={field.onChange}
                          data-testid="network-dhcp-switch"
                        />
                      </FormControl>
                    </FormItem>
                  )}
                />

                {!dhcpEnabled && (
                  <div className="animate-in fade-in slide-in-from-top-2 grid grid-cols-1 gap-[24px] md:grid-cols-2">
                    <FormField
                      control={form.control}
                      name="address"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel className="text-[#a1a1a6]">IP Address</FormLabel>
                          <FormControl>
                            <Input
                              {...field}
                              className="border-[#3a3a3c] bg-transparent text-white"
                              data-testid="network-ip-address-input"
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="prefixLength"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel className="text-[#a1a1a6]">Prefix Length</FormLabel>
                          <FormControl>
                            <Input
                              type="number"
                              {...field}
                              onChange={(e) => field.onChange(Number.parseInt(e.target.value))}
                              className="border-[#3a3a3c] bg-transparent text-white"
                              data-testid="network-prefix-length-input"
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="gateway"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel className="text-[#a1a1a6]">Gateway</FormLabel>
                          <FormControl>
                            <Input
                              {...field}
                              className="border-[#3a3a3c] bg-transparent text-white"
                              data-testid="network-gateway-input"
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                  </div>
                )}
              </SettingsCardContent>
            </SettingsCard>

            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(191,90,242,0.1)]">
                    <Server className="size-5 text-[#bf5af2]" />
                  </div>
                  <div>
                    <SettingsCardTitle>DNS Configuration</SettingsCardTitle>
                    <SettingsCardDescription>Domain Name System servers</SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="space-y-[24px]">
                <FormField
                  control={form.control}
                  name="dnsFromDHCP"
                  render={({ field }) => (
                    <FormItem className="flex flex-row items-center justify-between rounded-lg border border-[#3a3a3c] bg-[#2c2c2e] p-4">
                      <FormLabel className="text-base leading-none text-white">
                        Obtain DNS from DHCP
                      </FormLabel>
                      <FormControl>
                        <Switch
                          checked={field.value}
                          onCheckedChange={field.onChange}
                          data-testid="network-dns-from-dhcp-switch"
                        />
                      </FormControl>
                    </FormItem>
                  )}
                />
                {!dnsFromDHCP && (
                  <div className="animate-in fade-in slide-in-from-top-2 grid grid-cols-1 gap-[24px] md:grid-cols-2">
                    <FormField
                      control={form.control}
                      name="primaryDNS"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel className="text-[#a1a1a6]">Primary DNS</FormLabel>
                          <FormControl>
                            <Input
                              {...field}
                              className="border-[#3a3a3c] bg-transparent text-white"
                              data-testid="network-primary-dns-input"
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="secondaryDNS"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel className="text-[#a1a1a6]">Secondary DNS</FormLabel>
                          <FormControl>
                            <Input
                              {...field}
                              className="border-[#3a3a3c] bg-transparent text-white"
                              data-testid="network-secondary-dns-input"
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                  </div>
                )}
              </SettingsCardContent>
            </SettingsCard>

            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(48,209,88,0.1)]">
                    <Wifi className="size-5 text-[#30d158]" />
                  </div>
                  <div>
                    <SettingsCardTitle>Port Configuration</SettingsCardTitle>
                    <SettingsCardDescription>
                      HTTP and RTSP ports (reboot required)
                    </SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="space-y-[24px]">
                <div className="grid grid-cols-1 gap-[24px] md:grid-cols-2">
                  <FormField
                    control={form.control}
                    name="httpPort"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-[#a1a1a6]">HTTP Port</FormLabel>
                        <FormControl>
                          <Input
                            type="number"
                            {...field}
                            onChange={(e) => field.onChange(Number.parseInt(e.target.value))}
                            className="border-[#3a3a3c] bg-transparent text-white"
                            data-testid="network-http-port-input"
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="rtspPort"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-[#a1a1a6]">RTSP Port</FormLabel>
                        <FormControl>
                          <Input
                            type="number"
                            {...field}
                            onChange={(e) => field.onChange(Number.parseInt(e.target.value))}
                            className="border-[#3a3a3c] bg-transparent text-white"
                            data-testid="network-rtsp-port-input"
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                </div>
              </SettingsCardContent>
            </SettingsCard>

            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(48,209,88,0.1)]">
                    <Server className="size-5 text-[#30d158]" />
                  </div>
                  <div>
                    <SettingsCardTitle>SNMP</SettingsCardTitle>
                    <SettingsCardDescription>
                      Read-only SNMPv2c agent (applies without reboot)
                    </SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="space-y-[24px]">
                {snmpError && (
                  <p className="text-accent-red text-[13px]" data-testid="network-snmp-load-error">
                    {snmpLoadError instanceof Error
                      ? snmpLoadError.message
                      : 'Failed to load SNMP settings'}
                  </p>
                )}
                <FormField
                  control={form.control}
                  name="snmpEnabled"
                  render={({ field }) => (
                    <FormItem className="flex items-center justify-between gap-[16px]">
                      <div>
                        <FormLabel className="text-[#a1a1a6]">Enable SNMP</FormLabel>
                        <FormDescription className="text-[#636366]">
                          Default community &quot;public&quot; is insecure on untrusted networks
                        </FormDescription>
                      </div>
                      <FormControl>
                        <Switch
                          checked={field.value}
                          onCheckedChange={field.onChange}
                          disabled={snmpUnavailable}
                          data-testid="network-snmp-enabled-switch"
                        />
                      </FormControl>
                    </FormItem>
                  )}
                />
                <div className="grid grid-cols-1 gap-[24px] md:grid-cols-2">
                  <FormField
                    control={form.control}
                    name="snmpPort"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-[#a1a1a6]">SNMP Port</FormLabel>
                        <FormControl>
                          <Input
                            type="number"
                            {...field}
                            disabled={snmpUnavailable}
                            onChange={(e) => field.onChange(Number.parseInt(e.target.value))}
                            className="border-[#3a3a3c] bg-transparent text-white"
                            data-testid="network-snmp-port-input"
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="snmpCommunity"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-[#a1a1a6]">RO Community</FormLabel>
                        <FormControl>
                          <Input
                            {...field}
                            disabled={snmpUnavailable}
                            className="border-[#3a3a3c] bg-transparent text-white"
                            data-testid="network-snmp-community-input"
                          />
                        </FormControl>
                        <FormMessage data-testid="network-snmp-community-error" />
                      </FormItem>
                    )}
                  />
                </div>
              </SettingsCardContent>
            </SettingsCard>

            <div className="flex items-center gap-[16px]">
              <Button
                type="submit"
                disabled={mutation.isPending || !form.formState.isDirty}
                className="h-[44px] rounded-[8px] bg-[#007AFF] px-[32px] font-semibold text-white hover:bg-[#0066CC]"
                data-testid="network-save-button"
              >
                <Save className="mr-2 size-4" />
                Save Changes
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={handleReset}
                className="h-[44px] rounded-[8px] border-[#3a3a3c] bg-transparent px-[32px] text-[#a1a1a6] hover:bg-[#1c1c1e] hover:text-white"
                data-testid="network-reset-button"
              >
                <RotateCcw className="mr-2 size-4" />
                Reset
              </Button>
            </div>

            <div className="mt-[24px] flex gap-[12px] rounded-[8px] border border-[rgba(0,122,255,0.2)] bg-[rgba(0,122,255,0.05)] p-[16px]">
              <Info className="mt-[2px] size-5 flex-shrink-0 text-[#007AFF]" />
              <div>
                <p className="mb-[4px] text-[14px] font-medium text-[#007AFF]">
                  Network Information
                </p>
                <p className="text-[13px] text-[#a1a1a6]">
                  IP, DNS, gateway, and Wi-Fi changes are written to a pending overlay and apply
                  after reboot. Port changes also require a restart of the ONVIF service.
                  {httpPort !== config?.protocols.http
                    ? ` After reboot use port ${httpPort} for the WebUI.`
                    : ''}
                </p>
              </div>
            </div>
          </form>
        </Form>

        <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
          <AlertDialogContent
            className="border-[#3a3a3c] bg-[#1c1c1e] text-white"
            data-testid="network-confirm-dialog"
          >
            <AlertDialogHeader>
              <AlertDialogTitle className="text-white" data-testid="network-confirm-dialog-title">
                Save Network Settings?
              </AlertDialogTitle>
              <AlertDialogDescription className="text-[#a1a1a6]">
                {confirmDescription}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel
                className="border-[#3a3a3c] bg-transparent text-white hover:bg-[#2c2c2e]"
                data-testid="network-confirm-cancel-button"
              >
                Cancel
              </AlertDialogCancel>
              <AlertDialogAction
                onClick={handleConfirm}
                className="bg-[#007AFF] text-white hover:bg-[#0066CC]"
                data-testid="network-confirm-save-button"
              >
                {mutation.isPending ? 'Saving...' : 'Confirm Save'}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </div>
  );
}
