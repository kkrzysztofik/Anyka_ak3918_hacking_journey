/**
 * Maintenance Page
 *
 * Device maintenance: Reboot, Factory Reset.
 */

import React, { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Wrench, Power, RotateCcw, AlertTriangle, Loader2 } from 'lucide-react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
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
import { systemReboot, setSystemFactoryDefault, type FactoryDefaultType } from '@/services/maintenanceService'

type DialogType = 'reboot' | 'softReset' | 'hardReset' | null

export default function MaintenancePage() {
  const [activeDialog, setActiveDialog] = useState<DialogType>(null)

  // Reboot mutation
  const rebootMutation = useMutation({
    mutationFn: systemReboot,
    onSuccess: () => {
      toast.success('Reboot initiated', {
        description: 'The device will restart. This page may become unavailable.',
      })
      setActiveDialog(null)
    },
    onError: (error) => {
      toast.error('Failed to reboot', {
        description: error instanceof Error ? error.message : 'An error occurred',
      })
    },
  })

  // Factory reset mutation
  const resetMutation = useMutation({
    mutationFn: (type: FactoryDefaultType) => setSystemFactoryDefault(type),
    onSuccess: () => {
      toast.success('Factory reset initiated', {
        description: 'The device will reset and restart.',
      })
      setActiveDialog(null)
    },
    onError: (error) => {
      toast.error('Failed to reset', {
        description: error instanceof Error ? error.message : 'An error occurred',
      })
    },
  })

  const handleReboot = () => {
    rebootMutation.mutate()
  }

  const handleReset = (type: FactoryDefaultType) => {
    resetMutation.mutate(type)
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Maintenance</h1>

      {/* Device Operations */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Wrench className="w-5 h-5" />
            Device Operations
          </CardTitle>
          <CardDescription>
            Restart or reset the device
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between p-4 border rounded-lg">
            <div>
              <h3 className="font-medium">Reboot Device</h3>
              <p className="text-sm text-muted-foreground">
                Restart the device. Settings will be preserved.
              </p>
            </div>
            <Button
              variant="outline"
              onClick={() => setActiveDialog('reboot')}
            >
              <Power className="w-4 h-4 mr-2" />
              Reboot
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Danger Zone */}
      <Card className="border-destructive">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-destructive">
            <AlertTriangle className="w-5 h-5" />
            Danger Zone
          </CardTitle>
          <CardDescription>
            These actions are irreversible
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between p-4 border border-destructive/50 rounded-lg">
            <div>
              <h3 className="font-medium">Soft Reset</h3>
              <p className="text-sm text-muted-foreground">
                Reset settings to defaults. Network configuration is preserved.
              </p>
            </div>
            <Button
              variant="outline"
              className="border-destructive text-destructive hover:bg-destructive hover:text-destructive-foreground"
              onClick={() => setActiveDialog('softReset')}
            >
              <RotateCcw className="w-4 h-4 mr-2" />
              Soft Reset
            </Button>
          </div>

          <div className="flex items-center justify-between p-4 border border-destructive/50 rounded-lg">
            <div>
              <h3 className="font-medium">Factory Reset</h3>
              <p className="text-sm text-muted-foreground">
                Reset all settings including network. You may lose access.
              </p>
            </div>
            <Button
              variant="destructive"
              onClick={() => setActiveDialog('hardReset')}
            >
              <RotateCcw className="w-4 h-4 mr-2" />
              Factory Reset
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Reboot Confirmation */}
      <AlertDialog open={activeDialog === 'reboot'} onOpenChange={(open) => !open && setActiveDialog(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Reboot Device</AlertDialogTitle>
            <AlertDialogDescription>
              The device will restart. This page may become unavailable during the reboot process.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={handleReboot}>
              {rebootMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Reboot Now
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Soft Reset Confirmation */}
      <AlertDialog open={activeDialog === 'softReset'} onOpenChange={(open) => !open && setActiveDialog(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle className="flex items-center gap-2 text-destructive">
              <AlertTriangle className="w-5 h-5" />
              Soft Reset
            </AlertDialogTitle>
            <AlertDialogDescription>
              This will reset all settings except network configuration.
              The device will restart after reset.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => handleReset('Soft')}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {resetMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Confirm Reset
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Hard Reset Confirmation */}
      <AlertDialog open={activeDialog === 'hardReset'} onOpenChange={(open) => !open && setActiveDialog(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle className="flex items-center gap-2 text-destructive">
              <AlertTriangle className="w-5 h-5" />
              Factory Reset
            </AlertDialogTitle>
            <AlertDialogDescription>
              <strong>Warning:</strong> This will reset ALL settings including network configuration.
              You may permanently lose access to this device if you don't have physical access.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => handleReset('Hard')}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {resetMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Factory Reset
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
