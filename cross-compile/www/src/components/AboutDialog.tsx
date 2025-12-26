import React, { useEffect, useState } from 'react';

import { Code, Copyright, Info, Loader2 } from 'lucide-react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { type DeviceInfo, getDeviceInformation } from '@/services/deviceService';

interface AboutDialogProps {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

export function AboutDialog({ open, onOpenChange }: AboutDialogProps) {
  const [deviceInfo, setDeviceInfo] = useState<DeviceInfo | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    if (open && !deviceInfo) {
      const fetchData = async () => {
        setIsLoading(true);
        try {
          const info = await getDeviceInformation();
          setDeviceInfo(info);
        } catch (error) {
          console.error('Failed to fetch device info:', error);
          toast.error('Failed to load device information');
        } finally {
          setIsLoading(false);
        }
      };
      fetchData();
    }
  }, [open, deviceInfo]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="bg-dark-sidebar border-dark-border text-white sm:max-w-[425px]"
        data-testid="about-dialog-content"
      >
        <DialogHeader>
          <div className="mb-2 flex items-center gap-3">
            <div className="bg-accent-red/10 flex size-10 items-center justify-center rounded-lg">
              <Info className="text-accent-red size-6" />
            </div>
            <div>
              <DialogTitle className="text-xl" data-testid="about-dialog-title">
                About Device
              </DialogTitle>
              <DialogDescription
                className="text-dark-secondary-text"
                data-testid="about-dialog-description"
              >
                System information and firmware details
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="py-2">
          {(() => {
            if (isLoading) {
              return (
                <div
                  className="text-dark-secondary-text flex flex-col items-center justify-center py-8"
                  data-testid="about-loading"
                >
                  <Loader2 className="mb-2 h-8 w-8 animate-spin" />
                  <p>Loading device information...</p>
                </div>
              );
            }
            if (deviceInfo) {
              return (
                <div className="space-y-4" data-testid="about-content">
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div className="space-y-1">
                      <p className="text-dark-secondary-text">Manufacturer</p>
                      <p className="font-medium text-white" data-testid="about-manufacturer-value">
                        {deviceInfo.manufacturer}
                      </p>
                    </div>
                    <div className="space-y-1">
                      <p className="text-dark-secondary-text">Model</p>
                      <p className="font-medium text-white" data-testid="about-model-value">
                        {deviceInfo.model}
                      </p>
                    </div>
                    <div className="space-y-1">
                      <p className="text-dark-secondary-text">Firmware Version</p>
                      <p className="font-medium text-white" data-testid="about-firmware-value">
                        {deviceInfo.firmwareVersion}
                      </p>
                    </div>
                    <div className="space-y-1">
                      <p className="text-dark-secondary-text">Serial Number</p>
                      <p className="font-medium text-white" data-testid="about-serial-value">
                        {deviceInfo.serialNumber}
                      </p>
                    </div>
                    <div className="col-span-2 space-y-1">
                      <p className="text-dark-secondary-text">Hardware ID</p>
                      <p
                        className="bg-dark-bg border-dark-border rounded border p-2 font-mono text-xs font-medium break-all text-white"
                        data-testid="about-hardware-id-value"
                      >
                        {deviceInfo.hardwareId}
                      </p>
                    </div>
                  </div>

                  <div className="border-dark-border space-y-3 border-t pt-4">
                    <a
                      href="https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-dark-secondary-text flex items-center gap-2 text-sm transition-colors hover:text-white"
                      data-testid="about-github-link"
                    >
                      <Code className="size-4" />
                      <span>View Project on GitHub</span>
                    </a>

                    <div className="text-dark-secondary-text flex items-center gap-2 text-sm">
                      <Copyright className="size-4" />
                      <a
                        href="https://www.gnu.org/licenses/gpl-3.0.html"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="transition-colors hover:text-white"
                        data-testid="about-license-link"
                      >
                        GNU General Public License v3.0
                      </a>
                    </div>

                    <div className="pt-2 text-center">
                      <p
                        className="text-dark-secondary-text text-[11px]"
                        data-testid="about-copyright"
                      >
                        © 2025 Krzysztof Krzysztofik. All rights reserved.
                      </p>
                    </div>
                  </div>
                </div>
              );
            }
            return (
              <div
                className="text-dark-secondary-text py-8 text-center"
                data-testid="about-error-message"
              >
                Unable to load device information.
              </div>
            );
          })()}
        </div>

        <DialogFooter>
          <Button
            onClick={() => onOpenChange(false)}
            className="bg-accent-red hover:bg-accent-red/90 text-white"
            data-testid="about-close-button"
          >
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
