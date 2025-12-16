/**
 * About Device Dialog
 *
 * Modal showing read-only device information.
 */

import React from 'react'
import { Info } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import type { DeviceInfo } from '@/services/deviceService'

interface AboutDialogProps {
  deviceInfo: DeviceInfo | undefined
  isLoading?: boolean
}

export function AboutDialog({ deviceInfo, isLoading }: AboutDialogProps) {
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm">
          <Info className="w-4 h-4 mr-2" />
          About Device
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>About Device</DialogTitle>
          <DialogDescription>
            Hardware and firmware information
          </DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="py-4 text-center text-muted-foreground">
            Loading device information...
          </div>
        ) : deviceInfo ? (
          <dl className="grid grid-cols-2 gap-4 py-4">
            <div>
              <dt className="text-sm text-muted-foreground">Manufacturer</dt>
              <dd className="font-medium">{deviceInfo.manufacturer}</dd>
            </div>
            <div>
              <dt className="text-sm text-muted-foreground">Model</dt>
              <dd className="font-medium">{deviceInfo.model}</dd>
            </div>
            <div>
              <dt className="text-sm text-muted-foreground">Firmware Version</dt>
              <dd className="font-medium">{deviceInfo.firmwareVersion}</dd>
            </div>
            <div>
              <dt className="text-sm text-muted-foreground">Serial Number</dt>
              <dd className="font-medium">{deviceInfo.serialNumber}</dd>
            </div>
            <div className="col-span-2">
              <dt className="text-sm text-muted-foreground">Hardware ID</dt>
              <dd className="font-medium font-mono text-xs">{deviceInfo.hardwareId}</dd>
            </div>
          </dl>
        ) : (
          <div className="py-4 text-center text-destructive">
            Failed to load device information
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}
