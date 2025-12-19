import React, { useEffect, useState } from 'react'
import { Info, Loader2, Copyright, Github } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { getDeviceInformation, type DeviceInfo } from '@/services/deviceService'
import { toast } from 'sonner'

interface AboutDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function AboutDialog({ open, onOpenChange }: AboutDialogProps) {
  const [deviceInfo, setDeviceInfo] = useState<DeviceInfo | null>(null)
  const [isLoading, setIsLoading] = useState(false)

  useEffect(() => {
    if (open && !deviceInfo) {
      const fetchData = async () => {
        setIsLoading(true)
        try {
          const info = await getDeviceInformation()
          setDeviceInfo(info)
        } catch (error) {
          console.error('Failed to fetch device info:', error)
          toast.error('Failed to load device information')
        } finally {
          setIsLoading(false)
        }
      }
      fetchData()
    }
  }, [open, deviceInfo])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px] bg-dark-sidebar border-dark-border text-white">
        <DialogHeader>
          <div className="flex items-center gap-3 mb-2">
            <div className="size-10 bg-accent-red/10 rounded-lg flex items-center justify-center">
              <Info className="size-6 text-accent-red" />
            </div>
            <div>
              <DialogTitle className="text-xl">About Device</DialogTitle>
              <DialogDescription className="text-dark-secondary-text">
                System information and firmware details
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="py-2">
          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-8 text-dark-secondary-text">
              <Loader2 className="h-8 w-8 animate-spin mb-2" />
              <p>Loading device information...</p>
            </div>
          ) : deviceInfo ? (
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div className="space-y-1">
                  <p className="text-dark-secondary-text">Manufacturer</p>
                  <p className="font-medium text-white">{deviceInfo.manufacturer}</p>
                </div>
                <div className="space-y-1">
                  <p className="text-dark-secondary-text">Model</p>
                  <p className="font-medium text-white">{deviceInfo.model}</p>
                </div>
                <div className="space-y-1">
                  <p className="text-dark-secondary-text">Firmware Version</p>
                  <p className="font-medium text-white">{deviceInfo.firmwareVersion}</p>
                </div>
                <div className="space-y-1">
                  <p className="text-dark-secondary-text">Serial Number</p>
                  <p className="font-medium text-white">{deviceInfo.serialNumber}</p>
                </div>
                <div className="col-span-2 space-y-1">
                  <p className="text-dark-secondary-text">Hardware ID</p>
                  <p className="font-medium text-white font-mono text-xs bg-dark-bg p-2 rounded border border-dark-border break-all">
                    {deviceInfo.hardwareId}
                  </p>
                </div>
              </div>

              <div className="pt-4 border-t border-dark-border space-y-3">
                <a
                  href="https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-2 text-dark-secondary-text hover:text-white transition-colors text-sm"
                >
                  <Github className="size-4" />
                  <span>View Project on GitHub</span>
                </a>

                <div className="flex items-center gap-2 text-dark-secondary-text text-sm">
                  <Copyright className="size-4" />
                  <a
                    href="https://www.gnu.org/licenses/gpl-3.0.html"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="hover:text-white transition-colors"
                  >
                    GNU General Public License v3.0
                  </a>
                </div>

                <div className="pt-2 text-center">
                  <p className="text-dark-secondary-text text-[11px]">
                    © 2025 Krzysztof Krzysztofik. All rights reserved.
                  </p>
                </div>
              </div>
            </div>
          ) : (
            <div className="py-8 text-center text-dark-secondary-text">
              Unable to load device information.
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            onClick={() => onOpenChange(false)}
            className="bg-accent-red hover:bg-accent-red/90 text-white"
          >
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
