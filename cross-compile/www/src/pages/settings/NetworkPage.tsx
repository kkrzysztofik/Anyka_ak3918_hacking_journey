/**
 * Network Page
 *
 * Configure network interfaces, DNS, and ports.
 */
import React, { useEffect, useState } from 'react';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Globe, Info, Network, RotateCcw, Save, Server, Wifi } from 'lucide-react';
import { useForm, useWatch } from 'react-hook-form';
import { toast } from 'sonner';
import { z } from 'zod';

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
import {
  type NetworkConfig,
  getNetworkConfig,
  setDNS,
  setNetworkInterface,
} from '@/services/networkService';

// Validation Schema
const ipRegex =
  /^(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;

const networkSchema = z.object({
  dhcp: z.boolean(),
  address: z.string().regex(ipRegex, 'Invalid IP address').optional().or(z.literal('')),
  prefixLength: z.number().min(0).max(32).optional(),
  gateway: z.string().regex(ipRegex, 'Invalid IP address').optional().or(z.literal('')),
  dnsFromDHCP: z.boolean(),
  primaryDNS: z.string().regex(ipRegex, 'Invalid IP address').optional().or(z.literal('')),
  secondaryDNS: z.string().regex(ipRegex, 'Invalid IP address').optional().or(z.literal('')),
  // Stubs
  onvifDiscovery: z.boolean().default(true),
  hostname: z.string().optional(),
  httpPort: z.number().default(80),
  httpsPort: z.number().default(443),
  rtspPort: z.number().default(554),
});

type NetworkFormData = z.infer<typeof networkSchema>;

export default function NetworkPage() {
  const queryClient = useQueryClient();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [pendingValues, setPendingValues] = useState<NetworkFormData | null>(null);

  // Fetch data
  const { data: config, isLoading } = useQuery<NetworkConfig>({
    queryKey: ['networkConfig'],
    queryFn: getNetworkConfig,
  });

  // Form setup
  const form = useForm<NetworkFormData>({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    resolver: zodResolver(networkSchema) as any,
    defaultValues: {
      dhcp: true,
      address: '',
      prefixLength: 24,
      gateway: '',
      dnsFromDHCP: true,
      primaryDNS: '',
      secondaryDNS: '',
      onvifDiscovery: true,
      hostname: 'camera-device',
      httpPort: 80,
      httpsPort: 443,
      rtspPort: 554,
    },
  });

  // Watch for conditional rendering
  const dhcpEnabled = useWatch({ control: form.control, name: 'dhcp' });
  const dnsFromDHCP = useWatch({ control: form.control, name: 'dnsFromDHCP' });

  // Load data into form
  useEffect(() => {
    if (config) {
      const iface = config.interfaces[0]; // Assume single interface
      form.reset({
        dhcp: iface?.dhcp ?? true,
        address: iface?.address || '',
        prefixLength: iface?.prefixLength || 24,
        gateway: iface?.gateway || '',
        dnsFromDHCP: config.dns.fromDHCP,
        primaryDNS: config.dns.dnsServers[0] || '',
        secondaryDNS: config.dns.dnsServers[1] || '',
        // Keep stubs
        onvifDiscovery: true,
        hostname: iface?.name || 'camera-device',
        httpPort: 80,
        httpsPort: 443,
        rtspPort: 554,
      });
    }
  }, [config, form]);

  const mutation = useMutation({
    mutationFn: async (values: NetworkFormData) => {
      const iface = config?.interfaces[0];
      if (!iface) throw new Error('No interface found');

      await setNetworkInterface(iface.token, values.dhcp, values.address, values.prefixLength);

      const dnsServers = [values.primaryDNS, values.secondaryDNS].filter(Boolean) as string[];
      await setDNS(values.dnsFromDHCP, dnsServers);
    },
    onSuccess: () => {
      toast.success('Network settings saved', {
        description: 'The device may lose connectivity if IP settings changed.',
      });
      queryClient.invalidateQueries({ queryKey: ['networkConfig'] });
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
      // Re-trigger the useEffect to reset
      // A quick hack is to just re-fetch or clone the data,
      // but form.reset inside useEffect handles it if we depend on config.
      // Actually, we can just call form.reset with current config derived values
      const iface = config.interfaces[0];
      form.reset({
        dhcp: iface?.dhcp ?? true,
        address: iface?.address || '',
        prefixLength: iface?.prefixLength || 24,
        gateway: iface?.gateway || '',
        dnsFromDHCP: config.dns.fromDHCP,
        primaryDNS: config.dns.dnsServers[0] || '',
        secondaryDNS: config.dns.dnsServers[1] || '',
        onvifDiscovery: true,
        hostname: iface?.name || 'camera-device',
        httpPort: 80,
        httpsPort: 443,
        rtspPort: 554,
      });
      toast.info('Form reset to current values');
    }
  };

  if (isLoading) return <div className="text-white">Loading...</div>;

  const primaryInterface = config?.interfaces[0];

  return (
    <div
      className="absolute inset-0 overflow-auto bg-[#0d0d0d] lg:inset-[0_0_0_356.84px]"
      data-name="Container"
    >
      <div className="max-w-[1200px] p-[16px] pb-[80px] md:p-[32px] md:pb-[48px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px]">
          <h1 className="mb-[8px] text-[22px] text-white md:text-[28px]">Network</h1>
          <p className="text-[13px] text-[#a1a1a6] md:text-[14px]">
            Configure IP address, DNS, and service ports
          </p>
        </div>

        {/* Connection Status Card */}
        <StatusCard>
          <StatusCardImage>
            <Network className="size-8 opacity-50" />
          </StatusCardImage>
          <StatusCardContent>
            <StatusCardItem label="MAC Address" value={primaryInterface?.hwAddress || '--'} />
            <StatusCardItem label="Speed" value="100 Mbps" />
            <StatusCardItem
              label="Status"
              value={
                <div className="flex items-center gap-2">
                  <div className="size-2 rounded-full bg-green-500" />
                  <span>Connected</span>
                </div>
              }
            />
            <StatusCardItem label="Uptime" value="--" />
          </StatusCardContent>
        </StatusCard>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-[24px]">
            {/* Network Configuration */}
            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(10,132,255,0.1)]">
                    <Globe className="size-5 text-[#0a84ff]" />
                  </div>
                  <div>
                    <SettingsCardTitle>Network Configuration</SettingsCardTitle>
                    <SettingsCardDescription>
                      IP address and hostname settings
                    </SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="space-y-[24px]">
                {/* Hostname (Stub) */}
                <FormField
                  control={form.control}
                  name="hostname"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[#a1a1a6]">Hostname</FormLabel>
                      <FormControl>
                        <Input
                          {...field}
                          className="border-[#3a3a3c] bg-transparent text-white focus:border-[#dc2626]"
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                {/* DHCP Toggle */}
                <FormField
                  control={form.control}
                  name="dhcp"
                  render={({ field }) => (
                    <FormItem className="flex flex-row items-center justify-between rounded-lg border border-[#3a3a3c] bg-[#2c2c2e] p-4">
                      <div className="space-y-0.5">
                        <FormLabel className="text-base text-white">DHCP</FormLabel>
                        <FormDescription className="text-[#a1a1a6]">
                          Automatically obtain IP settings from the router
                        </FormDescription>
                      </div>
                      <FormControl>
                        <Switch checked={field.value} onCheckedChange={field.onChange} />
                      </FormControl>
                    </FormItem>
                  )}
                />

                {/* ONVIF Discovery (Stub) */}
                <FormField
                  control={form.control}
                  name="onvifDiscovery"
                  render={({ field }) => (
                    <FormItem className="flex flex-row items-center justify-between rounded-lg border border-[#3a3a3c] bg-[#2c2c2e] p-4">
                      <div className="space-y-0.5">
                        <FormLabel className="text-base text-white">ONVIF Discovery</FormLabel>
                        <FormDescription className="text-[#a1a1a6]">
                          Make this device visible to ONVIF clients
                        </FormDescription>
                      </div>
                      <FormControl>
                        <Switch checked={field.value} onCheckedChange={field.onChange} />
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
                              onChange={(e) => field.onChange(parseInt(e.target.value))}
                              className="border-[#3a3a3c] bg-transparent text-white"
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

            {/* DNS Configuration */}
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
                      <div className="space-y-0.5">
                        <FormLabel className="text-base text-white">Obtain DNS from DHCP</FormLabel>
                      </div>
                      <FormControl>
                        <Switch checked={field.value} onCheckedChange={field.onChange} />
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

            {/* Port Configuration (Stub) */}
            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(48,209,88,0.1)]">
                    <Wifi className="size-5 text-[#30d158]" />
                  </div>
                  <div>
                    <SettingsCardTitle>Port Configuration</SettingsCardTitle>
                    <SettingsCardDescription>Service ports (HTTP/RTSP)</SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="space-y-[24px]">
                <div className="grid grid-cols-1 gap-[24px] md:grid-cols-3">
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
                            onChange={(e) => field.onChange(parseInt(e.target.value))}
                            className="border-[#3a3a3c] bg-transparent text-white"
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="httpsPort"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-[#a1a1a6]">HTTPS Port</FormLabel>
                        <FormControl>
                          <Input
                            type="number"
                            {...field}
                            onChange={(e) => field.onChange(parseInt(e.target.value))}
                            className="border-[#3a3a3c] bg-transparent text-white"
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
                            onChange={(e) => field.onChange(parseInt(e.target.value))}
                            className="border-[#3a3a3c] bg-transparent text-white"
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                </div>
              </SettingsCardContent>
            </SettingsCard>

            {/* Action Buttons */}
            <div className="flex items-center gap-[16px]">
              <Button
                type="submit"
                disabled={mutation.isPending || !form.formState.isDirty}
                className="h-[44px] rounded-[8px] bg-[#dc2626] px-[32px] font-semibold text-white hover:bg-[#ef4444]"
              >
                <Save className="mr-2 size-4" />
                Save Changes
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={handleReset}
                className="h-[44px] rounded-[8px] border-[#3a3a3c] bg-transparent px-[32px] text-[#a1a1a6] hover:bg-[#1c1c1e] hover:text-white"
              >
                <RotateCcw className="mr-2 size-4" />
                Reset
              </Button>
            </div>

            {/* Help Text */}
            <div className="mt-[24px] flex gap-[12px] rounded-[8px] border border-[rgba(0,122,255,0.2)] bg-[rgba(0,122,255,0.05)] p-[16px]">
              <Info className="mt-[2px] size-5 flex-shrink-0 text-[#007AFF]" />
              <div>
                <p className="mb-[4px] text-[14px] font-medium text-[#007AFF]">
                  Network Information
                </p>
                <p className="text-[13px] text-[#a1a1a6]">
                  Changing IP settings may cause the device to become unreachable. Ensure you are on
                  the same subnet if you set a static IP address. Port changes will require a device
                  reboot.
                </p>
              </div>
            </div>
          </form>
        </Form>

        {/* Confirmation Modal */}
        <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
          <AlertDialogContent className="border-[#3a3a3c] bg-[#1c1c1e] text-white">
            <AlertDialogHeader>
              <AlertDialogTitle className="text-white">Save Network Settings?</AlertDialogTitle>
              <AlertDialogDescription className="text-[#a1a1a6]">
                Applying these changes might disconnect the device from the network. You may need to
                reconnect using the new IP address.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel className="border-[#3a3a3c] bg-transparent text-white hover:bg-[#2c2c2e]">
                Cancel
              </AlertDialogCancel>
              <AlertDialogAction
                onClick={handleConfirm}
                className="bg-[#dc2626] text-white hover:bg-[#ef4444]"
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
