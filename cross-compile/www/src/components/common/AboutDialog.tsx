/**
 * About Device Dialog
 *
 * Modal showing read-only device information.
 */
import React from 'react';

import { Info } from 'lucide-react';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import type { DeviceInfo } from '@/services/deviceService';

interface AboutDialogProps {
  readonly deviceInfo: DeviceInfo | undefined;
  readonly isLoading?: boolean;
}

export function AboutDialog({ deviceInfo, isLoading }: AboutDialogProps) {
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm">
          <Info className="mr-2 h-4 w-4" />
          About Device
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>About Device</DialogTitle>
          <DialogDescription>Hardware and firmware information</DialogDescription>
        </DialogHeader>

        {(() => {
          if (isLoading) {
            return (
              <div className="text-muted-foreground py-4 text-center">
                Loading device information...
              </div>
            );
          }
          if (deviceInfo) {
            return (
              <dl className="grid grid-cols-2 gap-4 py-4">
                <div>
                  <dt className="text-muted-foreground text-sm">Manufacturer</dt>
                  <dd className="font-medium">{deviceInfo.manufacturer}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground text-sm">Model</dt>
                  <dd className="font-medium">{deviceInfo.model}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground text-sm">Firmware Version</dt>
                  <dd className="font-medium">{deviceInfo.firmwareVersion}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground text-sm">Serial Number</dt>
                  <dd className="font-medium">{deviceInfo.serialNumber}</dd>
                </div>
                <div className="col-span-2">
                  <dt className="text-muted-foreground text-sm">Hardware ID</dt>
                  <dd className="font-mono text-xs font-medium">{deviceInfo.hardwareId}</dd>
                </div>
              </dl>
            );
          }
          return (
            <div className="text-destructive py-4 text-center">
              Failed to load device information
            </div>
          );
        })()}
      </DialogContent>
    </Dialog>
  );
}
