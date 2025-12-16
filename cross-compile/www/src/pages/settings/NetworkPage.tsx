/**
 * Network Settings Page
 *
 * Configure IP, DNS, and Ports with DHCP toggle.
 */

import React, { useState } from 'react'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Network, Save, Loader2, AlertTriangle } from 'lucide-react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { Form, FormControl, FormDescription, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { getNetworkConfig, setNetworkInterface, setDNS, type NetworkConfig } from '@/services/networkService'
import { networkSchema, type NetworkFormData } from '@/lib/schemas/network'

export default function NetworkPage() {
  const queryClient = useQueryClient()
  const [showWarning, setShowWarning] = useState(false)
  const [pendingValues, setPendingValues] = useState<NetworkFormData | null>(null)

  // Fetch network config
  const { data, isLoading, error } = useQuery<NetworkConfig>({
    queryKey: ['networkConfig'],
    queryFn: getNetworkConfig,
  })

  const form = useForm<NetworkFormData>({
    resolver: zodResolver(networkSchema),
    defaultValues: {
      dhcp: true,
      address: '',
      prefixLength: 24,
      gateway: '',
      dnsFromDHCP: true,
      dns1: '',
      dns2: '',
    },
  })

  const dhcpEnabled = form.watch('dhcp')
  const dnsFromDHCP = form.watch('dnsFromDHCP')

  // Update form when data is loaded
  React.useEffect(() => {
    if (data && data.interfaces.length > 0) {
      const iface = data.interfaces[0]
      form.reset({
        dhcp: iface.dhcp,
        address: iface.address || '',
        prefixLength: iface.prefixLength || 24,
        gateway: iface.gateway || '',
        dnsFromDHCP: data.dns.fromDHCP,
        dns1: data.dns.dnsServers[0] || '',
        dns2: data.dns.dnsServers[1] || '',
      })
    }
  }, [data, form])

  // Mutation for saving network config
  const mutation = useMutation({
    mutationFn: async (values: NetworkFormData) => {
      const token = data?.interfaces[0]?.token || 'eth0'

      // Set network interface
      await setNetworkInterface(
        token,
        values.dhcp,
        values.address,
        values.prefixLength
      )

      // Set DNS
      const dnsServers = [values.dns1, values.dns2].filter(Boolean) as string[]
      await setDNS(values.dnsFromDHCP, dnsServers)
    },
    onSuccess: () => {
      toast.success('Network settings saved', {
        description: 'Changes may require a device reboot to take effect',
      })
      queryClient.invalidateQueries({ queryKey: ['networkConfig'] })
    },
    onError: (error) => {
      toast.error('Failed to save network settings', {
        description: error instanceof Error ? error.message : 'An error occurred',
      })
    },
  })

  const handleSubmit = (values: NetworkFormData) => {
    // Show warning dialog before saving network changes
    setPendingValues(values)
    setShowWarning(true)
  }

  const confirmSave = () => {
    if (pendingValues) {
      mutation.mutate(pendingValues)
    }
    setShowWarning(false)
    setPendingValues(null)
  }

  if (error) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-bold">Network</h1>
        <Card>
          <CardContent className="pt-6">
            <p className="text-destructive">Failed to load network configuration</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Network</h1>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Network className="w-5 h-5" />
            Network Configuration
          </CardTitle>
          <CardDescription>
            Configure IP address, DHCP, and DNS settings
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground py-4">
              <Loader2 className="w-4 h-4 animate-spin" />
              Loading network settings...
            </div>
          ) : (
            <Form {...form}>
              <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-6">
                {/* Interface info */}
                {data?.interfaces[0] && (
                  <div className="grid grid-cols-2 gap-4 p-4 bg-muted rounded-lg text-sm">
                    <div>
                      <span className="text-muted-foreground">Interface:</span>
                      <span className="ml-2 font-medium">{data.interfaces[0].name}</span>
                    </div>
                    <div>
                      <span className="text-muted-foreground">MAC:</span>
                      <span className="ml-2 font-mono">{data.interfaces[0].hwAddress}</span>
                    </div>
                  </div>
                )}

                {/* DHCP Toggle */}
                <FormField
                  control={form.control}
                  name="dhcp"
                  render={({ field }) => (
                    <FormItem className="flex items-center justify-between rounded-lg border p-4">
                      <div className="space-y-0.5">
                        <FormLabel className="text-base">DHCP</FormLabel>
                        <FormDescription>
                          Automatically obtain IP address from network
                        </FormDescription>
                      </div>
                      <FormControl>
                        <Switch checked={field.value} onCheckedChange={field.onChange} />
                      </FormControl>
                    </FormItem>
                  )}
                />

                {/* Static IP fields (hidden when DHCP enabled) */}
                {!dhcpEnabled && (
                  <div className="grid gap-4 md:grid-cols-2">
                    <FormField
                      control={form.control}
                      name="address"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>IP Address</FormLabel>
                          <FormControl>
                            <Input placeholder="192.168.1.100" {...field} />
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
                          <FormLabel>Prefix Length (CIDR)</FormLabel>
                          <FormControl>
                            <Input type="number" min={1} max={32} {...field} />
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
                          <FormLabel>Gateway</FormLabel>
                          <FormControl>
                            <Input placeholder="192.168.1.1" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                  </div>
                )}

                {/* DNS Settings */}
                <div className="border-t pt-6 mt-6">
                  <h3 className="text-sm font-medium mb-4">DNS Configuration</h3>

                  <FormField
                    control={form.control}
                    name="dnsFromDHCP"
                    render={({ field }) => (
                      <FormItem className="flex items-center justify-between rounded-lg border p-4 mb-4">
                        <div className="space-y-0.5">
                          <FormLabel className="text-base">DNS from DHCP</FormLabel>
                          <FormDescription>
                            Use DNS servers provided by DHCP
                          </FormDescription>
                        </div>
                        <FormControl>
                          <Switch checked={field.value} onCheckedChange={field.onChange} />
                        </FormControl>
                      </FormItem>
                    )}
                  />

                  {!dnsFromDHCP && (
                    <div className="grid gap-4 md:grid-cols-2">
                      <FormField
                        control={form.control}
                        name="dns1"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Primary DNS</FormLabel>
                            <FormControl>
                              <Input placeholder="8.8.8.8" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                      <FormField
                        control={form.control}
                        name="dns2"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Secondary DNS</FormLabel>
                            <FormControl>
                              <Input placeholder="8.8.4.4" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>
                  )}
                </div>

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

      {/* Warning Dialog */}
      <AlertDialog open={showWarning} onOpenChange={setShowWarning}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle className="flex items-center gap-2">
              <AlertTriangle className="w-5 h-5 text-destructive" />
              Connectivity Risk
            </AlertDialogTitle>
            <AlertDialogDescription>
              Changing network settings may cause loss of connectivity to this device.
              Make sure you have physical access to the device in case you need to reset the network configuration.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirmSave}>
              Continue and Save
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
