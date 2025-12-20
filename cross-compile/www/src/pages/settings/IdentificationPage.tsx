/**
 * Identification Page
 *
 * Manage device identification and location.
 */
import React, { useEffect } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Activity, HardDrive, Image as ImageIcon, Info, RotateCcw, Save, Wifi } from 'lucide-react';
import { useForm } from 'react-hook-form';
import { toast } from 'sonner';

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
import {
  type DeviceIdentification,
  getDeviceIdentification,
  setDeviceInformation,
} from '@/services/deviceService';
import { type NetworkInterface, getNetworkInterfaces } from '@/services/networkService';

export default function IdentificationPage() {
  const queryClient = useQueryClient();

  // Fetch device info
  const { data: deviceInfo, isLoading: isDeviceLoading } = useQuery<DeviceIdentification>({
    queryKey: ['deviceInformation'],
    queryFn: getDeviceIdentification,
  });

  // Fetch network info for status card
  const { data: networkInterfaces } = useQuery<NetworkInterface[]>({
    queryKey: ['networkInterfaces'],
    queryFn: getNetworkInterfaces,
  });

  const form = useForm<DeviceIdentification>({
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
    },
  });

  // Update form when data is loaded
  useEffect(() => {
    if (deviceInfo) {
      form.reset(deviceInfo);
    }
  }, [deviceInfo, form]);

  // Mutation for saving device info
  const mutation = useMutation({
    mutationFn: setDeviceInformation,
    onSuccess: () => {
      toast.success('Device information saved');
      queryClient.invalidateQueries({ queryKey: ['deviceInformation'] });
    },
    onError: (error) => {
      toast.error('Failed to save device information', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  const onSubmit = (values: DeviceIdentification) => {
    mutation.mutate(values);
  };

  const handleReset = () => {
    if (deviceInfo) {
      form.reset(deviceInfo);
      toast.info('Form reset to current device values');
    }
  };

  if (isDeviceLoading) {
    return <div className="text-white">Loading...</div>;
  }

  const primaryInterface = networkInterfaces?.[0];

  return (
    <div
      className="absolute inset-0 overflow-auto bg-[#0d0d0d] lg:inset-[0_0_0_356.84px]"
      data-name="Container"
    >
      <div className="max-w-[1200px] p-[16px] pb-[80px] md:p-[32px] md:pb-[48px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px]">
          <h1 className="mb-[8px] text-[22px] text-white md:text-[28px]">Identification</h1>
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
                <div className="flex items-center gap-2">
                  <div className="size-2 rounded-full bg-green-500" />
                  <span>Online</span>
                </div>
              }
            />
            <StatusCardItem label="Uptime" value="--" /> {/* TODO: Implement uptime */}
            <StatusCardItem label="MAC Address" value={primaryInterface?.hwAddress || '--'} />
            <StatusCardItem label="Speed" value="100 Mbps" />
            <StatusCardItem label="Channel" value="Auto" />
            <StatusCardItem label="Security" value="--" />
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
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
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
                  <label className="mb-1 block text-[13px] text-[#6b6b6f]">Manufacturer</label>
                  <div className="text-[15px] text-white">
                    {deviceInfo?.deviceInfo.manufacturer}
                  </div>
                </div>
                <div>
                  <label className="mb-1 block text-[13px] text-[#6b6b6f]">Model</label>
                  <div className="text-[15px] text-white">{deviceInfo?.deviceInfo.model}</div>
                </div>
                <div>
                  <label className="mb-1 block text-[13px] text-[#6b6b6f]">Hardware ID</label>
                  <div className="text-[15px] text-white">{deviceInfo?.deviceInfo.hardwareId}</div>
                </div>
                <div>
                  <label className="mb-1 block text-[13px] text-[#6b6b6f]">Firmware Version</label>
                  <div className="text-[15px] text-white">
                    {deviceInfo?.deviceInfo.firmwareVersion}
                  </div>
                </div>
                <div>
                  <label className="mb-1 block text-[13px] text-[#6b6b6f]">Serial Number</label>
                  <div className="text-[15px] text-white">
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
                  <label className="mb-1 block text-[13px] text-[#6b6b6f]">Device ID</label>
                  <div className="text-[15px] text-white">{deviceInfo?.deviceInfo.hardwareId}</div>
                </div>
                <div>
                  <label className="mb-1 block text-[13px] text-[#6b6b6f]">IP Address</label>
                  <div className="text-[15px] text-white">{primaryInterface?.address || '--'}</div>
                </div>
                <div>
                  <label className="mb-1 block text-[13px] text-[#6b6b6f]">MAC Address</label>
                  <div className="text-[15px] text-white">
                    {primaryInterface?.hwAddress || '--'}
                  </div>
                </div>
                <div>
                  <label className="mb-1 block text-[13px] text-[#6b6b6f]">ONVIF Version</label>
                  <div className="text-[15px] text-white">24.12</div>
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
                Reset to Default
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
