/**
 * Time Settings Page
 *
 * Configure NTP vs Manual Time.
 */

import React from 'react'
import { useForm } from 'react-hook-form'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Clock, Save, Loader2 } from 'lucide-react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { Form, FormControl, FormDescription, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form'
import { getSystemDateAndTime, setSystemDateAndTime, type SystemDateTime, type DateTimeType } from '@/services/timeService'

interface TimeFormData {
  useNTP: boolean
  timezone: string
  daylightSavings: boolean
  manualDate: string
  manualTime: string
}

export default function TimePage() {
  const queryClient = useQueryClient()

  // Fetch time config
  const { data, isLoading, error } = useQuery<SystemDateTime>({
    queryKey: ['systemDateTime'],
    queryFn: getSystemDateAndTime,
    refetchInterval: 60000, // Refresh every minute
  })

  const form = useForm<TimeFormData>({
    defaultValues: {
      useNTP: true,
      timezone: 'UTC',
      daylightSavings: false,
      manualDate: '',
      manualTime: '',
    },
  })

  const useNTP = form.watch('useNTP')

  // Update form when data is loaded
  React.useEffect(() => {
    if (data) {
      const d = data.utcDateTime
      form.reset({
        useNTP: data.dateTimeType === 'NTP',
        timezone: data.timezone,
        daylightSavings: data.daylightSavings,
        manualDate: d.toISOString().split('T')[0],
        manualTime: d.toISOString().split('T')[1].substring(0, 5),
      })
    }
  }, [data, form])

  // Mutation for saving time config
  const mutation = useMutation({
    mutationFn: async (values: TimeFormData) => {
      const dateTimeType: DateTimeType = values.useNTP ? 'NTP' : 'Manual'
      let manualDateTime: Date | undefined

      if (!values.useNTP && values.manualDate && values.manualTime) {
        manualDateTime = new Date(`${values.manualDate}T${values.manualTime}:00Z`)
      }

      await setSystemDateAndTime(
        dateTimeType,
        values.daylightSavings,
        values.timezone,
        manualDateTime
      )
    },
    onSuccess: () => {
      toast.success('Time settings saved')
      queryClient.invalidateQueries({ queryKey: ['systemDateTime'] })
    },
    onError: (error) => {
      toast.error('Failed to save time settings', {
        description: error instanceof Error ? error.message : 'An error occurred',
      })
    },
  })

  const onSubmit = (values: TimeFormData) => {
    mutation.mutate(values)
  }

  // Format current time for display
  const currentTimeDisplay = data?.utcDateTime
    ? data.utcDateTime.toLocaleString()
    : '--'

  if (error) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-bold">Time</h1>
        <Card>
          <CardContent className="pt-6">
            <p className="text-destructive">Failed to load time configuration</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Time</h1>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Clock className="w-5 h-5" />
            Time Configuration
          </CardTitle>
          <CardDescription>
            Configure time synchronization and timezone
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground py-4">
              <Loader2 className="w-4 h-4 animate-spin" />
              Loading time settings...
            </div>
          ) : (
            <Form {...form}>
              <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
                {/* Current Time Display */}
                <div className="p-4 bg-muted rounded-lg">
                  <div className="text-sm text-muted-foreground">Current Device Time</div>
                  <div className="text-2xl font-mono font-bold">{currentTimeDisplay}</div>
                  <div className="text-sm text-muted-foreground mt-1">
                    Mode: {data?.dateTimeType || '--'} | Timezone: {data?.timezone || '--'}
                  </div>
                </div>

                {/* NTP Toggle */}
                <FormField
                  control={form.control}
                  name="useNTP"
                  render={({ field }) => (
                    <FormItem className="flex items-center justify-between rounded-lg border p-4">
                      <div className="space-y-0.5">
                        <FormLabel className="text-base">Use NTP</FormLabel>
                        <FormDescription>
                          Automatically synchronize time with network time servers
                        </FormDescription>
                      </div>
                      <FormControl>
                        <Switch checked={field.value} onCheckedChange={field.onChange} />
                      </FormControl>
                    </FormItem>
                  )}
                />

                {/* Timezone */}
                <FormField
                  control={form.control}
                  name="timezone"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Timezone</FormLabel>
                      <FormControl>
                        <Input placeholder="UTC+0" {...field} />
                      </FormControl>
                      <FormDescription>
                        POSIX timezone string (e.g., UTC+0, EST+5, PST+8)
                      </FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                {/* Daylight Savings */}
                <FormField
                  control={form.control}
                  name="daylightSavings"
                  render={({ field }) => (
                    <FormItem className="flex items-center justify-between rounded-lg border p-4">
                      <div className="space-y-0.5">
                        <FormLabel className="text-base">Daylight Saving Time</FormLabel>
                        <FormDescription>
                          Enable daylight saving time adjustment
                        </FormDescription>
                      </div>
                      <FormControl>
                        <Switch checked={field.value} onCheckedChange={field.onChange} />
                      </FormControl>
                    </FormItem>
                  )}
                />

                {/* Manual Date/Time (hidden when NTP enabled) */}
                {!useNTP && (
                  <div className="border-t pt-6 mt-6">
                    <h3 className="text-sm font-medium mb-4">Manual Date & Time</h3>
                    <div className="grid gap-4 md:grid-cols-2">
                      <FormField
                        control={form.control}
                        name="manualDate"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Date</FormLabel>
                            <FormControl>
                              <Input type="date" {...field} />
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
                            <FormLabel>Time (UTC)</FormLabel>
                            <FormControl>
                              <Input type="time" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>
                  </div>
                )}

                <div className="flex justify-end">
                  <Button type="submit" disabled={mutation.isPending || !form.formState.isDirty}>
                    {mutation.isPending ? (
                      <>
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        Saving...
                      </>
                    ) : (
                      <>
                        <Save className="mr-2 h-4 w-4" />
                        Save Changes
                      </>
                    )}
                  </Button>
                </div>
              </form>
            </Form>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
