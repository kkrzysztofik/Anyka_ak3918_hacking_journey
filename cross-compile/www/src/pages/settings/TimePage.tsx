import React, { useEffect, useState } from 'react';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Calendar, Clock, Globe, Monitor, RefreshCw, Save } from 'lucide-react';
import { useForm, useWatch } from 'react-hook-form';
import { toast } from 'sonner';
import { z } from 'zod';

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
import { Label } from '@/components/ui/label';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import {
  SettingsCard,
  SettingsCardContent,
  SettingsCardDescription,
  SettingsCardHeader,
  SettingsCardTitle,
} from '@/components/ui/settings-card';
import { Switch } from '@/components/ui/switch';
import { type DateTimeConfig, getDateTime, setDateTime, setNTP } from '@/services/timeService';

// Validation Schema
const timeSchema = z.object({
  mode: z.enum(['ntp', 'computer', 'manual']),
  ntpFromDHCP: z.boolean(),
  ntpServer1: z.string().optional(),
  ntpServer2: z.string().optional(),
  timezone: z.string().min(1, 'Timezone is required'),
  manualDate: z.string().optional(),
  manualTime: z.string().optional(),
});

type TimeFormData = z.infer<typeof timeSchema>;

// Common Timezones (Stub list - in a real app this would be extensive)
const TIMEZONES = [
  { value: 'GMT', label: 'GMT (Greenwich Mean Time)' },
  { value: 'CET', label: 'CET (Central European Time)' },
  { value: 'EST', label: 'EST (Eastern Standard Time)' },
  { value: 'PST', label: 'PST (Pacific Standard Time)' },
  { value: 'CST', label: 'CST (China Standard Time)' },
  { value: 'JST', label: 'JST (Japan Standard Time)' },
  { value: 'UTC', label: 'UTC (Coordinated Universal Time)' },
];

export default function TimePage() {
  const queryClient = useQueryClient();
  const [deviceTime, setDeviceTime] = useState<Date | null>(null);

  // Fetch Time Config
  const { data: config, isLoading } = useQuery<DateTimeConfig>({
    queryKey: ['timeConfig'],
    queryFn: getDateTime,
  });

  // Simulated live clock for "Current Device Time"
  useEffect(() => {
    if (config) {
      // Initialize with fetched time (parsing simplified for demo)
      // In reality, we'd offset this by local execution time
      const now = new Date();
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setDeviceTime(now);
    }
  }, [config]);

  useEffect(() => {
    const timer = setInterval(() => {
      setDeviceTime(new Date());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  const form = useForm<TimeFormData>({
    resolver: zodResolver(timeSchema),
    defaultValues: {
      mode: 'ntp',
      ntpFromDHCP: true,
      ntpServer1: 'pool.ntp.org',
      ntpServer2: 'time.google.com',
      timezone: 'UTC',
      manualDate: new Date().toISOString().split('T')[0],
      manualTime: new Date().toTimeString().split(' ')[0],
    },
  });

  const mode = useWatch({ control: form.control, name: 'mode' });
  const ntpFromDHCP = useWatch({ control: form.control, name: 'ntpFromDHCP' });

  // Load initial values
  useEffect(() => {
    if (config) {
      form.reset({
        mode: config.ntp.enabled ? 'ntp' : 'manual',
        ntpFromDHCP: config.ntp.fromDHCP,
        ntpServer1: 'pool.ntp.org', // Stub as API generally doesn't return server list easily in simple calls
        ntpServer2: 'time.google.com',
        timezone: config.timezone || 'UTC',
        manualDate: new Date().toISOString().split('T')[0],
        manualTime: new Date().toTimeString().split(' ')[0],
      });
    }
  }, [config, form]);

  const mutation = useMutation({
    mutationFn: async (values: TimeFormData) => {
      // 1. Set Timezone
      // await setTimezone(values.timezone) // If separate API existed

      // 2. Set Mode
      if (values.mode === 'ntp') {
        await setNTP(values.ntpFromDHCP);
        // If we supported setting custom NTP servers, we'd do it here
      } else if (values.mode === 'computer') {
        const now = new Date();
        await setDateTime('manual', now.toISOString(), values.timezone);
      } else {
        // Manual
        const dateStr = `${values.manualDate}T${values.manualTime}`;
        const date = new Date(dateStr);
        await setDateTime('manual', date.toISOString(), values.timezone);
      }
    },
    onSuccess: () => {
      toast.success('Time settings saved');
      queryClient.invalidateQueries({ queryKey: ['timeConfig'] });
    },
    onError: (error) => {
      toast.error('Failed to save time settings', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  const onSubmit = (values: TimeFormData) => {
    mutation.mutate(values);
  };

  const handleSyncComputer = () => {
    form.setValue('mode', 'computer');
    const now = new Date();
    form.setValue('manualDate', now.toISOString().split('T')[0]);
    form.setValue('manualTime', now.toTimeString().split(' ')[0]);
    toast.info('Selected computer time');
  };

  if (isLoading) return <div className="text-white">Loading...</div>;

  return (
    <div
      className="absolute inset-0 overflow-auto bg-[#0d0d0d] lg:inset-[0_0_0_356.84px]"
      data-name="Container"
    >
      <div className="max-w-[1200px] p-[16px] pb-[80px] md:p-[32px] md:pb-[48px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px]">
          <h1 className="mb-[8px] text-[22px] text-white md:text-[28px]">Time</h1>
          <p className="text-[13px] text-[#a1a1a6] md:text-[14px]">
            Configure system clock, NTP synchronization, and timezone
          </p>
        </div>

        {/* Current Time Display */}
        <div className="mb-[24px] flex items-center justify-between rounded-[16px] border border-[#3a3a3c] bg-gradient-to-r from-[#1c1c1e] to-[#2c2c2e] p-[24px]">
          <div>
            <div className="mb-[4px] text-[13px] font-medium tracking-wider text-[#a1a1a6] uppercase">
              Device Time
            </div>
            <div className="font-mono text-[32px] font-medium tracking-tight text-white">
              {deviceTime ? deviceTime.toLocaleTimeString() : '--:--:--'}
            </div>
            <div className="text-[14px] text-[#a1a1a6]">
              {deviceTime
                ? deviceTime.toLocaleDateString(undefined, {
                    weekday: 'long',
                    year: 'numeric',
                    month: 'long',
                    day: 'numeric',
                  })
                : 'Loading...'}
            </div>
          </div>
          <div className="flex size-[48px] items-center justify-center rounded-full bg-[#0a84ff]/10">
            <Clock className="size-6 text-[#0a84ff]" />
          </div>
        </div>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-[24px]">
            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(255,159,10,0.1)]">
                    <RefreshCw className="size-5 text-[#ff9f0a]" />
                  </div>
                  <div>
                    <SettingsCardTitle>Synchronization</SettingsCardTitle>
                    <SettingsCardDescription>
                      Choose how the device keeps time
                    </SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent className="space-y-[24px]">
                <FormField
                  control={form.control}
                  name="mode"
                  render={({ field }) => (
                    <RadioGroup
                      onValueChange={field.onChange}
                      defaultValue={field.value}
                      className="grid grid-cols-1 gap-[16px] md:grid-cols-3"
                    >
                      {/* NTP Mode */}
                      <div>
                        <RadioGroupItem
                          value="ntp"
                          id="ntp"
                          className="peer sr-only"
                          data-testid="time-page-ntp-radio-input"
                        />
                        <Label
                          htmlFor="ntp"
                          className="border-muted bg-popover hover:bg-accent hover:text-accent-foreground [&:has([data-state=checked])]:border-primary flex flex-col items-center justify-between rounded-md border-2 p-4 peer-data-[state=checked]:border-[#0a84ff] peer-data-[state=checked]:bg-[#0a84ff]/5"
                          data-testid="time-page-ntp-radio"
                        >
                          <Globe className="mb-3 h-6 w-6" />
                          NTP Server
                          <p className="mt-1 text-center text-[11px] font-normal text-[#a1a1a6]">
                            Automatic sync
                          </p>
                        </Label>
                      </div>

                      {/* Computer Mode */}
                      <div>
                        <RadioGroupItem
                          value="computer"
                          id="computer"
                          className="peer sr-only"
                          data-testid="time-page-computer-radio-input"
                        />
                        <Label
                          htmlFor="computer"
                          onClick={handleSyncComputer}
                          className="border-muted bg-popover hover:bg-accent hover:text-accent-foreground [&:has([data-state=checked])]:border-primary flex flex-col items-center justify-between rounded-md border-2 p-4 peer-data-[state=checked]:border-[#0a84ff] peer-data-[state=checked]:bg-[#0a84ff]/5"
                          data-testid="time-page-computer-radio"
                        >
                          <Monitor className="mb-3 h-6 w-6" />
                          Computer
                          <p className="mt-1 text-center text-[11px] font-normal text-[#a1a1a6]">
                            Sync with browser
                          </p>
                        </Label>
                      </div>

                      {/* Manual Mode */}
                      <div>
                        <RadioGroupItem
                          value="manual"
                          id="manual"
                          className="peer sr-only"
                          data-testid="time-page-manual-radio-input"
                        />
                        <Label
                          htmlFor="manual"
                          className="border-muted bg-popover hover:bg-accent hover:text-accent-foreground [&:has([data-state=checked])]:border-primary flex flex-col items-center justify-between rounded-md border-2 p-4 peer-data-[state=checked]:border-[#0a84ff] peer-data-[state=checked]:bg-[#0a84ff]/5"
                          data-testid="time-page-manual-radio"
                        >
                          <Calendar className="mb-3 h-6 w-6" />
                          Manual
                          <p className="mt-1 text-center text-[11px] font-normal text-[#a1a1a6]">
                            Set manually
                          </p>
                        </Label>
                      </div>
                    </RadioGroup>
                  )}
                />

                {/* NTP Settings */}
                {mode === 'ntp' && (
                  <div className="animate-in fade-in slide-in-from-top-2 space-y-[16px] pt-[8px]">
                    <FormField
                      control={form.control}
                      name="ntpFromDHCP"
                      render={({ field }) => (
                        <FormItem className="flex flex-row items-center justify-between rounded-lg border border-[#3a3a3c] bg-[#2c2c2e] p-4">
                          <div className="space-y-0.5">
                            <FormLabel className="text-base text-white">NTP from DHCP</FormLabel>
                            <FormDescription className="text-[#a1a1a6]">
                              Obtain NTP servers automatically
                            </FormDescription>
                          </div>
                          <FormControl>
                            <Switch
                              checked={field.value}
                              onCheckedChange={field.onChange}
                              data-testid="time-page-ntp-from-dhcp-switch"
                            />
                          </FormControl>
                        </FormItem>
                      )}
                    />

                    {!ntpFromDHCP && (
                      <div className="grid grid-cols-1 gap-[16px] md:grid-cols-2">
                        <FormField
                          control={form.control}
                          name="ntpServer1"
                          render={({ field }) => (
                            <FormItem>
                              <FormLabel className="text-[#a1a1a6]">Primary Server</FormLabel>
                              <FormControl>
                                <Input
                                  {...field}
                                  className="border-[#3a3a3c] bg-transparent text-white"
                                  data-testid="time-page-ntp-server1-input"
                                />
                              </FormControl>
                              <FormMessage />
                            </FormItem>
                          )}
                        />
                        <FormField
                          control={form.control}
                          name="ntpServer2"
                          render={({ field }) => (
                            <FormItem>
                              <FormLabel className="text-[#a1a1a6]">Secondary Server</FormLabel>
                              <FormControl>
                                <Input
                                  {...field}
                                  className="border-[#3a3a3c] bg-transparent text-white"
                                  data-testid="time-page-ntp-server2-input"
                                />
                              </FormControl>
                              <FormMessage />
                            </FormItem>
                          )}
                        />
                      </div>
                    )}
                  </div>
                )}

                {/* Manual Settings */}
                {mode === 'manual' && (
                  <div className="animate-in fade-in slide-in-from-top-2 grid grid-cols-1 gap-[16px] pt-[8px] md:grid-cols-2">
                    <FormField
                      control={form.control}
                      name="manualDate"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel className="text-[#a1a1a6]">Date</FormLabel>
                          <FormControl>
                            <Input
                              type="date"
                              {...field}
                              className="border-[#3a3a3c] bg-transparent text-white"
                              data-testid="time-page-manual-date-input"
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="manualTime"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel className="text-[#a1a1a6]">Time</FormLabel>
                          <FormControl>
                            <Input
                              type="time"
                              step="1"
                              {...field}
                              className="border-[#3a3a3c] bg-transparent text-white"
                              data-testid="time-page-manual-time-input"
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

            {/* Timezone Configuration */}
            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(191,90,242,0.1)]">
                    <Globe className="size-5 text-[#bf5af2]" />
                  </div>
                  <div>
                    <SettingsCardTitle>Time Zone</SettingsCardTitle>
                    <SettingsCardDescription>Set the local time zone</SettingsCardDescription>
                  </div>
                </div>
              </SettingsCardHeader>
              <SettingsCardContent>
                <FormField
                  control={form.control}
                  name="timezone"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[#a1a1a6]">Region</FormLabel>
                      <FormControl>
                        <select
                          className="placeholder:text-muted-foreground flex h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white focus:ring-2 focus:ring-[#0a84ff] focus:ring-offset-2 focus:outline-none disabled:cursor-not-allowed disabled:opacity-50"
                          onChange={field.onChange}
                          value={field.value}
                          data-testid="time-page-timezone-select"
                        >
                          <option value="" disabled>
                            Select timezone
                          </option>
                          {TIMEZONES.map((tz) => (
                            <option key={tz.value} value={tz.value}>
                              {tz.label}
                            </option>
                          ))}
                        </select>
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </SettingsCardContent>
            </SettingsCard>

            {/* Actions */}
            <div className="flex items-center gap-[16px]">
              <Button
                type="submit"
                disabled={mutation.isPending || !form.formState.isDirty}
                className="h-[44px] rounded-[8px] bg-[#0a84ff] px-[32px] font-semibold text-white hover:bg-[#0077ed]"
                data-testid="time-page-save-button"
              >
                <Save className="mr-2 size-4" />
                Save Changes
              </Button>
            </div>
          </form>
        </Form>
      </div>
    </div>
  );
}
