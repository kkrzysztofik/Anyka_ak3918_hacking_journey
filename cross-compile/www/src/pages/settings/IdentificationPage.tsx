/**
 * Identification Page
 *
 * Manage device identification and location.
 */
import React, { useEffect, useState } from 'react';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Activity,
  Globe,
  HardDrive,
  Image as ImageIcon,
  Info,
  Plus,
  Radar,
  RotateCcw,
  Save,
  Trash2,
  Wifi,
} from 'lucide-react';
import { useFieldArray, useForm } from 'react-hook-form';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Form,
  FormControl,
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
  type IdentificationFormData,
  type IdentificationFormInput,
  identificationSchema,
} from '@/lib/schemas/identification';
import {
  type DeviceIdentification,
  type DiscoveryMode,
  type Scope,
  getDeviceIdentification,
  getDiscoveryMode,
  getHostname,
  getScopes,
  scopesForSave,
  setDiscoveryMode,
  setHostname,
  setScopes,
} from '@/services/deviceService';
import { HealthStatusValue } from '@/components/settings/HealthStatusValue';
import { useDeviceStatus } from '@/hooks/useDeviceStatus';
import { handleMutationError } from '@/utils/errorHandling';

export default function IdentificationPage() {
  const queryClient = useQueryClient();

  // Fetch device info
  const { data: deviceInfo, isLoading: isDeviceLoading } = useQuery<DeviceIdentification>({
    queryKey: ['deviceInformation'],
    queryFn: getDeviceIdentification,
  });

  const { data: scopes } = useQuery<Scope[]>({
    queryKey: ['deviceScopes'],
    queryFn: getScopes,
  });

  const { data: hostname } = useQuery<string>({
    queryKey: ['hostname'],
    queryFn: getHostname,
  });

  const { data: discoveryMode } = useQuery<DiscoveryMode>({
    queryKey: ['discoveryMode'],
    queryFn: getDiscoveryMode,
  });

  const {
    healthStatus,
    primaryInterface,
    systemUptime,
    wifiChannel,
    wifiQuality,
    wifiSecurity,
  } = useDeviceStatus();

  const form = useForm<IdentificationFormInput, unknown, IdentificationFormData>({
    resolver: zodResolver(identificationSchema),
    defaultValues: {
      deviceInfo: {
        manufacturer: '',
        model: '',
        firmwareVersion: '',
        serialNumber: '',
        hardwareId: '',
      },
      name: '',
      location: '',
      hostname: '',
      discoveryMode: 'Discoverable',
      scopes: [],
    },
  });

  const { fields, append, remove } = useFieldArray({
    control: form.control,
    name: 'scopes',
  });
  const [newScope, setNewScope] = useState('');

  useEffect(() => {
    if (deviceInfo && scopes && hostname !== undefined && discoveryMode) {
      form.reset({
        ...deviceInfo,
        hostname,
        discoveryMode,
        scopes,
      });
    }
  }, [deviceInfo, scopes, hostname, discoveryMode, form]);

  const mutation = useMutation({
    mutationFn: async (values: IdentificationFormData) => {
      await setScopes(
        scopesForSave(values.scopes, { name: values.name, location: values.location }),
      );
      if (form.formState.dirtyFields.hostname) {
        await setHostname(values.hostname);
      }
    },
    onSuccess: () => {
      toast.success('Device information saved');
      queryClient.invalidateQueries({ queryKey: ['deviceInformation'] });
      queryClient.invalidateQueries({ queryKey: ['deviceScopes'] });
      queryClient.invalidateQueries({ queryKey: ['hostname'] });
    },
    onError: (error) => {
      handleMutationError(error, 'Failed to save device information');
    },
  });

  const discoveryMutation = useMutation({
    mutationFn: (mode: DiscoveryMode) => setDiscoveryMode(mode),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['discoveryMode'] });
      toast.success('Discovery mode updated');
    },
    onError: (error) => {
      handleMutationError(error, 'Failed to update discovery mode');
    },
  });

  const onSubmit = (values: IdentificationFormData) => {
    mutation.mutate(values);
  };

  const handleReset = () => {
    if (deviceInfo && scopes && hostname !== undefined && discoveryMode) {
      form.reset({
        ...deviceInfo,
        hostname,
        discoveryMode,
        scopes,
      });
      toast.info('Form reset to current device values');
    }
  };

  const handleAddScope = () => {
    const scopeItem = newScope.trim();
    if (!scopeItem) {
      return;
    }
    if (fields.some((field) => field.scopeItem === scopeItem)) {
      setNewScope('');
      return;
    }
    append({ scopeDef: 'Configurable', scopeItem });
    setNewScope('');
  };

  if (isDeviceLoading) {
    return (
      <div className="text-white" data-testid="identification-loading">
        Loading...
      </div>
    );
  }

  return (
    <div
      className="absolute inset-0 overflow-auto bg-[#0d0d0d] lg:inset-[0_0_0_356.84px]"
      data-name="Container"
    >
      <div className="max-w-[1200px] p-[16px] pb-[80px] md:p-[32px] md:pb-[48px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px]">
          <h1
            className="mb-[8px] text-[22px] text-white md:text-[28px]"
            data-testid="identification-title"
          >
            Identification
          </h1>
          <p className="text-[13px] text-[#a1a1a6] md:text-[14px]">
            View and configure device identification settings
          </p>
        </div>

        {/* Device Status Card */}
        <StatusCard>
          <StatusCardImage>
            <ImageIcon className="size-8 opacity-50" />
          </StatusCardImage>
          <StatusCardContent>
            <StatusCardItem label="Device Name" value={deviceInfo?.name || 'Unknown'} />
            <StatusCardItem label="Model" value={deviceInfo?.deviceInfo.model || 'Unknown'} />
            <StatusCardItem
              label="Status"
              value={
                <HealthStatusValue
                  label={healthStatus.label}
                  tone={healthStatus.tone}
                  detail={healthStatus.detail}
                  testId="identification-status-health"
                />
              }
            />
            <StatusCardItem
              label="Uptime"
              value={systemUptime}
              data-testid="identification-status-uptime"
            />
            <StatusCardItem
              label="MAC Address"
              value={primaryInterface?.hwAddress || '—'}
              data-testid="identification-status-mac"
            />
            <StatusCardItem
              label="Link Quality"
              value={wifiQuality}
              data-testid="identification-status-quality"
            />
            <StatusCardItem
              label="Channel"
              value={wifiChannel}
              data-testid="identification-status-channel"
            />
            <StatusCardItem
              label="Security"
              value={wifiSecurity}
              data-testid="identification-status-security"
            />
          </StatusCardContent>
        </StatusCard>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-[24px]">
            {/* Device Configuration */}
            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(220,38,38,0.1)]">
                    <Activity className="size-5 text-[#dc2626]" />
                  </div>
                  <div>
                    <SettingsCardTitle>Device Configuration</SettingsCardTitle>
                    <SettingsCardDescription>
                      Configure basic device identity
                    </SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="space-y-[24px]">
                <FormField
                  control={form.control}
                  name="name"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[#a1a1a6]">Device Name</FormLabel>
                      <FormControl>
                        <Input
                          {...field}
                          className="border-[#3a3a3c] bg-transparent text-white focus:border-[#dc2626]"
                          data-testid="identification-device-name-input"
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="location"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[#a1a1a6]">Location</FormLabel>
                      <FormControl>
                        <Input
                          {...field}
                          className="border-[#3a3a3c] bg-transparent text-white focus:border-[#dc2626]"
                          data-testid="identification-device-location-input"
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
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
                          data-testid="identification-hostname-input"
                        />
                      </FormControl>
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
                    <Radar className="size-5 text-[#0a84ff]" />
                  </div>
                  <div>
                    <SettingsCardTitle>Discovery</SettingsCardTitle>
                    <SettingsCardDescription>
                      Control whether this camera answers WS-Discovery
                    </SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="space-y-[16px]">
                <div className="flex items-center justify-between gap-[16px]">
                  <div>
                    <p className="text-[14px] text-white">Discoverable</p>
                    <p className="text-[13px] text-[#a1a1a6]">
                      NonDiscoverable stops Probe replies and Hello announcements, so ONVIF clients
                      will not find this camera on the network.
                    </p>
                  </div>
                  <Switch
                    checked={discoveryMode === 'Discoverable'}
                    disabled={discoveryMutation.isPending || discoveryMode === undefined}
                    onCheckedChange={(enabled) => {
                      discoveryMutation.mutate(enabled ? 'Discoverable' : 'NonDiscoverable');
                    }}
                    data-testid="identification-discovery-switch"
                  />
                </div>
              </SettingsCardContent>
            </SettingsCard>

            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(48,209,88,0.1)]">
                    <Globe className="size-5 text-[#30d158]" />
                  </div>
                  <div>
                    <SettingsCardTitle>Scopes</SettingsCardTitle>
                    <SettingsCardDescription>
                      ONVIF scopes announced during discovery
                    </SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="p-0">
                <div className="overflow-hidden">
                  <table className="w-full text-left text-sm text-[#a1a1a6]">
                    <thead className="border-b border-[#3a3a3c] bg-[#1c1c1e] text-xs font-medium uppercase">
                      <tr>
                        <th className="px-6 py-4">Scope</th>
                        <th className="px-6 py-4">Type</th>
                        <th className="px-6 py-4 text-right">Actions</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-[#3a3a3c]">
                      {fields.map((field, index) => (
                        <tr
                          key={field.id}
                          className="transition-colors hover:bg-[#2c2c2e]/50"
                          data-testid={`identification-scope-row-${field.scopeItem}`}
                        >
                          <td className="px-6 py-4 font-mono text-[13px] break-all text-white">
                            {field.scopeItem}
                          </td>
                          <td className="px-6 py-4">
                            <Badge
                              className={`pointer-events-none rounded-md border px-2 py-1 text-xs font-medium ${
                                field.scopeDef === 'Fixed'
                                  ? 'border-[rgba(142,142,147,0.2)] bg-[rgba(142,142,147,0.1)] text-[#8e8e93]'
                                  : 'border-[rgba(48,209,88,0.2)] bg-[rgba(48,209,88,0.1)] text-[#30d158]'
                              }`}
                            >
                              {field.scopeDef}
                            </Badge>
                          </td>
                          <td className="px-6 py-4 text-right">
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              disabled={field.scopeDef === 'Fixed'}
                              onClick={() => remove(index)}
                              className="h-8 w-8 text-[#a1a1a6] hover:bg-[rgba(220,38,38,0.1)] hover:text-[#dc2626]"
                              data-testid={`identification-scope-remove-${field.scopeItem}`}
                            >
                              <Trash2 className="size-4" />
                            </Button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                <div className="flex gap-[8px] border-t border-[#3a3a3c] p-[16px]">
                  <Input
                    value={newScope}
                    onChange={(event) => setNewScope(event.target.value)}
                    placeholder="onvif://www.onvif.org/…"
                    className="border-[#3a3a3c] bg-transparent text-white focus:border-[#dc2626]"
                    data-testid="identification-scope-add-input"
                  />
                  <Button
                    type="button"
                    variant="outline"
                    onClick={handleAddScope}
                    className="h-[40px] border-[#3a3a3c] bg-transparent text-[#a1a1a6] hover:bg-[#1c1c1e] hover:text-white"
                    data-testid="identification-scope-add-button"
                  >
                    <Plus className="mr-2 size-4" />
                    Add
                  </Button>
                </div>
              </SettingsCardContent>
            </SettingsCard>

            {/* Hardware Information */}
            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(255,159,10,0.1)]">
                    <HardDrive className="size-5 text-[#ff9f0a]" />
                  </div>
                  <div>
                    <SettingsCardTitle>Hardware Information</SettingsCardTitle>
                    <SettingsCardDescription>Read-only hardware details</SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="grid grid-cols-1 gap-[24px] md:grid-cols-2">
                <div>
                  <p className="mb-1 block text-[13px] text-[#6b6b6f]">Manufacturer</p>
                  <div className="font-mono text-[15px] text-white">
                    {deviceInfo?.deviceInfo.manufacturer}
                  </div>
                </div>
                <div>
                  <p className="mb-1 block text-[13px] text-[#6b6b6f]">Model</p>
                  <div className="font-mono text-[15px] text-white">
                    {deviceInfo?.deviceInfo.model}
                  </div>
                </div>
                <div>
                  <p className="mb-1 block text-[13px] text-[#6b6b6f]">Hardware ID</p>
                  <div className="font-mono text-[15px] text-white">
                    {deviceInfo?.deviceInfo.hardwareId}
                  </div>
                </div>
                <div>
                  <p className="mb-1 block text-[13px] text-[#6b6b6f]">Firmware Version</p>
                  <div className="font-mono text-[15px] text-white">
                    {deviceInfo?.deviceInfo.firmwareVersion}
                  </div>
                </div>
                <div>
                  <p className="mb-1 block text-[13px] text-[#6b6b6f]">Serial Number</p>
                  <div className="font-mono text-[15px] text-white">
                    {deviceInfo?.deviceInfo.serialNumber}
                  </div>
                </div>
              </SettingsCardContent>
            </SettingsCard>

            {/* Network & System Information */}
            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(0,122,255,0.1)]">
                    <Wifi className="size-5 text-[#007AFF]" />
                  </div>
                  <div>
                    <SettingsCardTitle>Network & System Information</SettingsCardTitle>
                    <SettingsCardDescription>Network connectivity details</SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="grid grid-cols-1 gap-[24px] md:grid-cols-2">
                <div>
                  <p className="mb-1 block text-[13px] text-[#6b6b6f]">Device ID</p>
                  <div className="font-mono text-[15px] text-white">
                    {deviceInfo?.deviceInfo.hardwareId}
                  </div>
                </div>
                <div>
                  <p className="mb-1 block text-[13px] text-[#6b6b6f]">IP Address</p>
                  <div className="font-mono text-[15px] text-white">
                    {primaryInterface?.address || '--'}
                  </div>
                </div>
                <div>
                  <p className="mb-1 block text-[13px] text-[#6b6b6f]">MAC Address</p>
                  <div className="font-mono text-[15px] text-white">
                    {primaryInterface?.hwAddress || '--'}
                  </div>
                </div>
                <div>
                  <p className="mb-1 block text-[13px] text-[#6b6b6f]">ONVIF Version</p>
                  <div className="font-mono text-[15px] text-white">24.12</div>
                </div>
              </SettingsCardContent>
            </SettingsCard>

            {/* Action Buttons */}
            <div className="flex items-center gap-[16px]">
              <Button
                type="submit"
                disabled={mutation.isPending || !form.formState.isDirty}
                className="h-[44px] rounded-[8px] bg-[#dc2626] px-[32px] font-semibold text-white hover:bg-[#ef4444]"
                data-testid="identification-save-button"
              >
                <Save className="mr-2 size-4" />
                Save Changes
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={handleReset}
                className="h-[44px] rounded-[8px] border-[#3a3a3c] bg-transparent px-[32px] text-[#a1a1a6] hover:bg-[#1c1c1e] hover:text-white"
                data-testid="identification-reset-button"
              >
                <RotateCcw className="mr-2 size-4" />
                Discard Changes
              </Button>
            </div>

            {/* Help Text */}
            <div className="mt-[24px] flex gap-[12px] rounded-[8px] border border-[rgba(0,122,255,0.2)] bg-[rgba(0,122,255,0.05)] p-[16px]">
              <Info className="mt-[2px] size-5 flex-shrink-0 text-[#007AFF]" />
              <div>
                <p className="mb-[4px] text-[14px] font-medium text-[#007AFF]">
                  Device Information
                </p>
                <p className="text-[13px] text-[#a1a1a6]">
                  The information provided here is used to identify this device on the network and
                  in the ONVIF client ecosystem. Changing the device name will update how it appears
                  in discovery tools.
                </p>
              </div>
            </div>
          </form>
        </Form>
      </div>
    </div>
  );
}
