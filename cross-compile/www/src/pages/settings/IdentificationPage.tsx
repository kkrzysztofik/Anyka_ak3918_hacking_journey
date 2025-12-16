/**
 * Identification Settings Page
 *
 * View and edit device identity (Name, Location) and see read-only HW info.
 */

import React from 'react'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { User, Save, Loader2 } from 'lucide-react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Form, FormControl, FormDescription, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form'
import { AboutDialog } from '@/components/common/AboutDialog'
import { getDeviceIdentification, setScopes, type DeviceIdentification } from '@/services/deviceService'
import { identificationSchema, type IdentificationFormData } from '@/lib/schemas/identification'

export default function IdentificationPage() {
  const queryClient = useQueryClient()

  // Fetch device identification
  const { data, isLoading, error } = useQuery<DeviceIdentification>({
    queryKey: ['deviceIdentification'],
    queryFn: getDeviceIdentification,
  })

  const form = useForm<IdentificationFormData>({
    resolver: zodResolver(identificationSchema),
    defaultValues: {
      name: '',
      location: '',
    },
  })

  // Update form when data is loaded
  React.useEffect(() => {
    if (data) {
      form.reset({
        name: data.name,
        location: data.location,
      })
    }
  }, [data, form])

  // Mutation for saving scopes
  const mutation = useMutation({
    mutationFn: (values: IdentificationFormData) => setScopes(values.name, values.location || ''),
    onSuccess: () => {
      toast.success('Settings saved', {
        description: 'Device identification updated successfully',
      })
      queryClient.invalidateQueries({ queryKey: ['deviceIdentification'] })
    },
    onError: (error) => {
      toast.error('Failed to save settings', {
        description: error instanceof Error ? error.message : 'An error occurred',
      })
    },
  })

  const onSubmit = (values: IdentificationFormData) => {
    mutation.mutate(values)
  }

  if (error) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-bold">Identification</h1>
        <Card>
          <CardContent className="pt-6">
            <p className="text-destructive">Failed to load device identification</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Identification</h1>
        <AboutDialog deviceInfo={data?.deviceInfo} isLoading={isLoading} />
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <User className="w-5 h-5" />
            Device Identity
          </CardTitle>
          <CardDescription>
            Configure the device name and location for identification
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground py-4">
              <Loader2 className="w-4 h-4 animate-spin" />
              Loading settings...
            </div>
          ) : (
            <Form {...form}>
              <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
                <FormField
                  control={form.control}
                  name="name"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Device Name</FormLabel>
                      <FormControl>
                        <Input placeholder="My Camera" {...field} />
                      </FormControl>
                      <FormDescription>
                        A friendly name to identify this device
                      </FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                  control={form.control}
                  name="location"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Location</FormLabel>
                      <FormControl>
                        <Input placeholder="Front Door" {...field} />
                      </FormControl>
                      <FormDescription>
                        Physical location of the device (optional)
                      </FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                {/* Read-only device info */}
                {data?.deviceInfo && (
                  <div className="border-t pt-6 mt-6">
                    <h3 className="text-sm font-medium text-muted-foreground mb-4">Hardware Information</h3>
                    <dl className="grid grid-cols-2 gap-4 text-sm">
                      <div>
                        <dt className="text-muted-foreground">Model</dt>
                        <dd className="font-medium">{data.deviceInfo.model}</dd>
                      </div>
                      <div>
                        <dt className="text-muted-foreground">Firmware</dt>
                        <dd className="font-medium">{data.deviceInfo.firmwareVersion}</dd>
                      </div>
                      <div>
                        <dt className="text-muted-foreground">Serial Number</dt>
                        <dd className="font-medium">{data.deviceInfo.serialNumber}</dd>
                      </div>
                      <div>
                        <dt className="text-muted-foreground">Manufacturer</dt>
                        <dd className="font-medium">{data.deviceInfo.manufacturer}</dd>
                      </div>
                    </dl>
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
