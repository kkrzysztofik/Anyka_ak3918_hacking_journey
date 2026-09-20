import React, { useEffect, useState } from 'react';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Calendar, Clock, Globe, RefreshCw, Save } from 'lucide-react';
import { useForm, useWatch } from 'react-hook-form';
import { toast } from 'sonner';
import { z } from 'zod';

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
import { Label } from '@/components/ui/label';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import {
  SettingsCard,
  SettingsCardContent,
  SettingsCardDescription,
  SettingsCardHeader,
  SettingsCardTitle,
} from '@/components/ui/settings-card';
import { getDiagnostics } from '@/services/diagnosticsService';
import {
  type DateTimeConfig,
  getDateTime,
  getNtp,
  setDateTime,
  setNtp,
  setSystemDateAndTime,
} from '@/services/timeService';
import { TIMEZONES } from '@/utils/timezones';

// Validation Schema
//
// The server requirement is object-level and NTP-only on purpose. A field-level
// rule would also fire in Manual mode, where the NTP panel is hidden — so a
// camera that reports no servers (or a failed GetNTP) would block the operator
// from saving a manual time against an error they cannot even see.
//
// It is a refinement rather than `min(1)` because an empty textarea yields
// [''], which `min(1)` accepts; the submit handler then drops the blank and
// calls setNtp([]), turning a field error into a save error.
const timeSchema = z
  .object({
    mode: z.enum(['ntp', 'manual']),
    ntpServers: z.array(z.string()),
    timezone: z.string().min(1, 'Timezone is required'),
    manualDate: z.string().optional(),
    manualTime: z.string().optional(),
  })
  .superRefine((values, ctx) => {
    if (values.mode === 'ntp' && !values.ntpServers.some((s) => s.trim().length > 0)) {
      ctx.addIssue({
        code: 'custom',
        path: ['ntpServers'],
        message: 'At least one NTP server is required',
      });
    }
  });

type TimeFormData = z.infer<typeof timeSchema>;

export default function TimePage() {
  const queryClient = useQueryClient();

  // The camera's clock, not ours. Capturing the offset once and ticking from
  // it is what makes a camera stuck at 1970 visible instead of showing the
  // operator their own correct browser clock. Prefer the camera's own
  // local time; it already has the camera's zone applied, which Intl cannot
  // reproduce from a POSIX TZ string.
  const [offsetMs, setOffsetMs] = useState<number | null>(null);
  const [now, setNow] = useState(() => Date.now());

  // Fetch Time Config
  const { data: config, isLoading } = useQuery<DateTimeConfig>({
    queryKey: ['timeConfig'],
    queryFn: getDateTime,
  });

  // The servers the camera actually uses, as it reports them.
  const ntpQuery = useQuery({
    queryKey: ['ntpServers'],
    queryFn: getNtp,
  });

  const { data: diagnostics } = useQuery({
    queryKey: ['diagnostics'],
    queryFn: (ctx) => getDiagnostics(ctx.signal),
    refetchInterval: 15000,
  });

  useEffect(() => {
    if (config) {
      const src = config.localDateTime ?? config.utcDateTime;
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setOffsetMs(src.getTime() - Date.now());
    }
  }, [config]);

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const deviceTime = offsetMs === null ? null : new Date(now + offsetMs);
  const clockIsStale = deviceTime !== null && deviceTime.getUTCFullYear() < 2020;

  const form = useForm<TimeFormData>({
    resolver: zodResolver(timeSchema),
    defaultValues: {
      mode: 'ntp',
      // No literal default: a server the camera does not use must never sit
      // in this form, or saving mid-load would write it to anyka.toml.
      ntpServers: [],
      timezone: 'UTC',
      manualDate: new Date().toISOString().split('T')[0],
      manualTime: new Date().toTimeString().split(' ')[0],
    },
  });

  const mode = useWatch({ control: form.control, name: 'mode' });

  // Load initial values.
  //
  // Gated on the server query too, not just the config: they are independent
  // SOAP calls, and resetting on whichever lands first would seed the form
  // with a placeholder server list, then wipe the operator's edits when the
  // real one arrives. `isPending` is false on error as well, so a failed
  // GetNTP still loads the rest of the page — with an empty list that zod
  // refuses to save.
  useEffect(() => {
    if (config && !ntpQuery.isPending) {
      form.reset({
        mode: config.ntp.enabled ? 'ntp' : 'manual',
        ntpServers: ntpQuery.data ?? [],
        timezone: config.timezone || 'UTC',
        manualDate: new Date().toISOString().split('T')[0],
        manualTime: new Date().toTimeString().split(' ')[0],
      });
    }
  }, [config, form, ntpQuery.data, ntpQuery.isPending]);

  // Warn once when the camera reports that no sync has completed.
  //
  // A null `time` block is ambiguous — NTP disabled, status file missing, or
  // an older supervisor that never publishes one — so it is not evidence of a
  // failed sync. Only a present block with no `last_sync_unix` says that.
  const warnedNoSync = React.useRef(false);
  useEffect(() => {
    if (
      config?.ntp.enabled &&
      diagnostics?.time &&
      diagnostics.time.last_sync_unix === null &&
      !warnedNoSync.current
    ) {
      warnedNoSync.current = true;
      toast.error('No NTP sync has completed yet');
    }
  }, [config, diagnostics]);

  const mutation = useMutation({
    mutationFn: async (values: TimeFormData) => {
      if (values.mode === 'ntp') {
        const servers = values.ntpServers.map((s) => s.trim()).filter((s) => s.length > 0);
        // The server list goes to the supervisor ([time].servers); the NTP
        // type clears the manual-clock marker in the camera's network config.
        await setNtp(servers);
        await setSystemDateAndTime('NTP', config?.daylightSavings ?? false, values.timezone);
      } else {
        // Manual
        const dateStr = `${values.manualDate}T${values.manualTime}`;
        const date = new Date(dateStr);
        await setDateTime(date.toISOString(), values.timezone, config?.daylightSavings ?? false);
      }
    },
    onSuccess: () => {
      toast.success('Time settings saved');
      queryClient.invalidateQueries({ queryKey: ['timeConfig'] });
      queryClient.invalidateQueries({ queryKey: ['ntpServers'] });
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

  // "Use Computer Time" fills the manual fields from the browser clock and
  // selects Manual; the ordinary Save flow applies them.
  //
  // It deliberately does not write on its own. Doing so discarded the
  // setDateTime promise, so a rejected write still reported success, never
  // invalidated `timeConfig`, and switched the camera to Manual while the
  // form still showed NTP selected.
  const handleSyncComputer = React.useCallback(() => {
    const now = new Date();
    const opts = { shouldDirty: true } as const;
    form.setValue('mode', 'manual', opts);
    form.setValue('manualDate', now.toISOString().split('T')[0], opts);
    form.setValue('manualTime', now.toTimeString().split(' ')[0], opts);
  }, [form]);

  if (isLoading)
    return (
      <div className="text-white" data-testid="time-loading">
        Loading...
      </div>
    );

  return (
    <div
      className="absolute inset-0 overflow-auto bg-[#0d0d0d] lg:inset-[0_0_0_356.84px]"
      data-name="Container"
    >
      <div className="max-w-[1200px] p-[16px] pb-[80px] md:p-[32px] md:pb-[48px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px]">
          <h1 className="mb-[8px] text-[22px] text-white md:text-[28px]" data-testid="time-title">
            Time
          </h1>
          <p className="text-[13px] text-[#a1a1a6] md:text-[14px]">
            Configure system clock, NTP synchronization, and timezone
          </p>
        </div>

        {/* Current Time Display */}
        <div className="mb-[24px] flex items-center justify-between rounded-[16px] border border-[#3a3a3c] bg-gradient-to-r from-[#1c1c1e] to-[#2c2c2e] p-[24px]">
          <div>
            <div
              className="mb-[4px] text-[13px] font-medium tracking-wider text-[#a1a1a6] uppercase"
              data-testid="time-device-time-label"
            >
              Device Time
            </div>
            <div
              className="font-mono text-[32px] font-medium tracking-tight text-white"
              data-testid="time-device-clock"
            >
              {deviceTime
                ? deviceTime.toLocaleTimeString('en-GB', { hour12: false, timeZone: 'UTC' })
                : '--:--:--'}
            </div>
            <div className="text-[14px] text-[#a1a1a6]">
              {deviceTime
                ? deviceTime.toLocaleDateString('en-GB', {
                    weekday: 'long',
                    year: 'numeric',
                    month: 'long',
                    day: 'numeric',
                    timeZone: 'UTC',
                  })
                : 'Loading...'}
            </div>
          </div>
          <div className="flex size-[48px] items-center justify-center rounded-full bg-[#0a84ff]/10">
            <Clock className="size-6 text-[#0a84ff]" />
          </div>
        </div>

        {clockIsStale && (
          <div
            className="mb-[24px] rounded-[12px] border border-[#ff453a] bg-[#ff453a]/10 p-[16px] text-[14px] text-[#ff453a]"
            data-testid="time-clock-stale"
            role="alert"
          >
            The camera's clock is not set. Authenticated requests will fail until NTP syncs.
          </div>
        )}

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-[24px]">
            <SettingsCard>
              <SettingsCardHeader>
                <div className="flex items-center gap-[12px]">
                  <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(255,159,10,0.1)]">
                    <RefreshCw className="size-5 text-[#ff9f0a]" />
                  </div>
                  <div>
                    <SettingsCardTitle data-testid="time-synchronization-title">
                      Synchronization
                    </SettingsCardTitle>
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
                      className="grid grid-cols-1 gap-[16px] md:grid-cols-2"
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

                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={handleSyncComputer}
                  data-testid="time-page-use-computer-time"
                >
                  Use Computer Time
                </Button>

                {/* NTP Settings */}
                {mode === 'ntp' && (
                  <div className="animate-in fade-in slide-in-from-top-2 space-y-[16px] pt-[8px]">
                    <FormField
                      control={form.control}
                      name="ntpServers"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel className="text-[#a1a1a6]">
                            NTP Servers (one per line)
                          </FormLabel>
                          <FormControl>
                            <textarea
                              value={field.value.join('\n')}
                              onChange={(e) => field.onChange(e.target.value.split('\n'))}
                              rows={3}
                              className="border-[#3a3a3c] bg-transparent p-3 font-mono text-white"
                              data-testid="time-page-ntp-servers-input"
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    <div
                      className="space-y-[4px] rounded-[12px] border border-[#3a3a3c] bg-[#2c2c2e] p-4"
                      data-testid="time-sync-status"
                      aria-live="polite"
                    >
                      {/* Three states, not two: an absent block means the
                          camera cannot tell us (NTP off, no status file, older
                          supervisor), which is not the same as a sync that has
                          never succeeded. */}
                      {!diagnostics?.time ? (
                        <div className="text-[14px] text-[#a1a1a6]" data-testid="time-sync-unknown">
                          Sync status unavailable.
                        </div>
                      ) : diagnostics.time.last_sync_unix === null ? (
                        <div className="text-[14px] text-[#ff9f0a]" data-testid="time-no-sync">
                          No NTP sync has completed yet.
                        </div>
                      ) : (
                        <>
                          <div className="text-[14px] text-white" data-testid="time-sync-state">
                            Last sync:{' '}
                            {new Date(diagnostics.time.last_sync_unix * 1000).toLocaleString()}
                            {diagnostics.time.last_server
                              ? ` via ${diagnostics.time.last_server}`
                              : ''}
                          </div>
                          {diagnostics.time.last_delta_s !== null && (
                            <div
                              className="text-[13px] text-[#a1a1a6]"
                              data-testid="time-sync-offset"
                            >
                              Clock offset: {diagnostics.time.last_delta_s >= 0 ? '+' : ''}
                              {diagnostics.time.last_delta_s}s
                            </div>
                          )}
                        </>
                      )}
                    </div>
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
                    <SettingsCardTitle data-testid="time-timezone-title">
                      Time Zone
                    </SettingsCardTitle>
                    <SettingsCardDescription data-testid="time-timezone-description">
                      Set the local time zone
                    </SettingsCardDescription>
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
