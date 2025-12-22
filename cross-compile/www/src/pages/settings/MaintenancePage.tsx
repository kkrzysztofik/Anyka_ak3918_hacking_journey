/**
 * Maintenance Page
 *
 * System maintenance operations (Reset, Reboot, Backup, Upgrade).
 */
import React, { useState } from 'react';

import { useMutation } from '@tanstack/react-query';
import {
  AlertTriangle,
  Archive,
  Download,
  FileUp,
  HardDrive,
  Power,
  RefreshCcw,
  Upload,
} from 'lucide-react';
import { toast } from 'sonner';

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import {
  SettingsCard,
  SettingsCardContent,
  SettingsCardDescription,
  SettingsCardHeader,
  SettingsCardTitle,
} from '@/components/ui/settings-card';
import {
  getSystemBackup,
  restoreSystem,
  setSystemFactoryDefault,
  systemReboot,
} from '@/services/maintenanceService';

export default function MaintenancePage() {
  const [rebootOpen, setRebootOpen] = useState(false);
  const [softResetOpen, setSoftResetOpen] = useState(false);
  const [hardResetOpen, setHardResetOpen] = useState(false);

  // Reboot Mutation
  const rebootMutation = useMutation({
    mutationFn: systemReboot,
    onSuccess: () => {
      setRebootOpen(false);
      toast.success('Device is rebooting', {
        description: 'Please wait for the device to restart...',
        duration: 10000,
      });
    },
    onError: (error) => {
      toast.error('Failed to reboot device', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
      setRebootOpen(false);
    },
  });

  // Soft Reset Mutation
  const softResetMutation = useMutation({
    mutationFn: () => setSystemFactoryDefault('Soft'),
    onSuccess: () => {
      setSoftResetOpen(false);
      toast.success('Device is resetting', {
        description: 'Settings returned to defaults. Device will reboot.',
        duration: 10000,
      });
    },
    onError: (error) => {
      toast.error('Failed to reset settings', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
      setSoftResetOpen(false);
    },
  });

  // Hard Reset Mutation
  const hardResetMutation = useMutation({
    mutationFn: () => setSystemFactoryDefault('Hard'),
    onSuccess: () => {
      setHardResetOpen(false);
      toast.success('Factory reset initiated', {
        description: 'All data will be erased. Device will reboot.',
        duration: 15000,
      });
    },
    onError: (error) => {
      toast.error('Failed to factory reset', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
      setHardResetOpen(false);
    },
  });

  // Backup Mutation
  const backupMutation = useMutation({
    mutationFn: getSystemBackup,
    onSuccess: (backupFiles) => {
      if (backupFiles.length === 0) {
        toast.error('Backup failed', {
          description: 'No backup files received from device',
        });
        return;
      }

      // Find config.toml file
      const configFile = backupFiles.find((f) => f.Name === 'config.toml');
      if (!configFile) {
        toast.error('Backup failed', {
          description: 'Backup does not contain config.toml',
        });
        return;
      }

      // Decode base64 to TOML content
      try {
        const tomlContent = atob(configFile.Data);
        const blob = new Blob([tomlContent], { type: 'text/plain' });
        const url = URL.createObjectURL(blob);

        // Generate filename with current date
        const date = new Date().toISOString().split('T')[0];
        const filename = `config-backup-${date}.toml`;

        // Trigger download
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        link.remove();

        // Clean up
        URL.revokeObjectURL(url);

        toast.success('Backup downloaded', {
          description: `Configuration saved as ${filename}`,
        });
      } catch (error) {
        toast.error('Backup failed', {
          description: error instanceof Error ? error.message : 'Failed to decode backup data',
        });
      }
    },
    onError: (error) => {
      toast.error('Backup failed', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  // Restore Mutation
  const restoreMutation = useMutation({
    mutationFn: restoreSystem,
    onSuccess: () => {
      toast.success('Configuration restored', {
        description: 'Device settings have been restored successfully',
      });
    },
    onError: (error) => {
      toast.error('Restore failed', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  // Backup handler
  const handleBackup = () => {
    backupMutation.mutate();
  };

  // Restore handler
  const handleRestore = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.toml';
    input.onchange = (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (file) {
        restoreMutation.mutate(file);
      }
    };
    input.click();
  };

  // Stub handler for Upgrade
  const handleUpgrade = () => toast.info('Firmware upgrade not available');

  return (
    <div
      className="absolute inset-0 overflow-auto bg-[#0d0d0d] lg:inset-[0_0_0_356.84px]"
      data-name="Container"
    >
      <div className="max-w-[1200px] p-[16px] pb-[80px] md:p-[32px] md:pb-[48px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px]">
          <h1 className="mb-[8px] text-[22px] text-white md:text-[28px]">Maintenance</h1>
          <p className="text-[13px] text-[#a1a1a6] md:text-[14px]">
            Manage system updates, backups, and device power state
          </p>
        </div>

        <div className="grid grid-cols-1 gap-[24px] lg:grid-cols-2">
          {/* Configuration Backup & Restore (STUB) */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(10,132,255,0.1)]">
                  <Archive className="size-5 text-[#0a84ff]" />
                </div>
                <div>
                  <SettingsCardTitle>Configuration Backup & Restore</SettingsCardTitle>
                  <SettingsCardDescription>Save or load device settings</SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent className="space-y-[16px]">
              <div className="grid grid-cols-2 gap-[16px]">
                <Button
                  variant="outline"
                  className="flex h-[80px] flex-col items-center justify-center gap-[8px] border-[#3a3a3c] bg-[#1c1c1e] hover:bg-[#2c2c2e] hover:text-white"
                  onClick={handleBackup}
                  disabled={backupMutation.isPending}
                >
                  <Download className="size-5 text-[#0a84ff]" />
                  <span>{backupMutation.isPending ? 'Backing up...' : 'Backup'}</span>
                </Button>
                <Button
                  variant="outline"
                  className="flex h-[80px] flex-col items-center justify-center gap-[8px] border-[#3a3a3c] bg-[#1c1c1e] hover:bg-[#2c2c2e] hover:text-white"
                  onClick={handleRestore}
                  disabled={restoreMutation.isPending}
                >
                  <Upload className="size-5 text-[#0a84ff]" />
                  <span>{restoreMutation.isPending ? 'Restoring...' : 'Restore'}</span>
                </Button>
              </div>
            </SettingsCardContent>
          </SettingsCard>

          {/* Firmware (STUB) */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(48,209,88,0.1)]">
                  <HardDrive className="size-5 text-[#30d158]" />
                </div>
                <div>
                  <SettingsCardTitle>Firmware</SettingsCardTitle>
                  <SettingsCardDescription>System version and updates</SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent className="space-y-[16px]">
              <div className="flex items-center justify-between rounded-[8px] bg-[#2c2c2e] p-[12px]">
                <div>
                  <div className="text-[12px] text-[#a1a1a6]">Current Version</div>
                  <div className="font-mono text-white">v1.0.0-beta</div>
                </div>
                <div className="flex items-center gap-1 text-[12px] text-[#30d158]">
                  <div className="size-2 rounded-full bg-[#30d158]" />
                  Up to date
                </div>
              </div>
              <Button
                className="w-full bg-[#30d158] font-semibold text-black hover:bg-[#28c840]"
                onClick={handleUpgrade}
              >
                <FileUp className="mr-2 size-4" />
                Check for Updates
              </Button>
            </SettingsCardContent>
          </SettingsCard>

          {/* Soft Reset */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(255,159,10,0.1)]">
                  <RefreshCcw className="size-5 text-[#ff9f0a]" />
                </div>
                <div>
                  <SettingsCardTitle>Soft factory reset</SettingsCardTitle>
                  <SettingsCardDescription>
                    Reset settings but keep IP configuration
                  </SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent>
              <AlertDialog open={softResetOpen} onOpenChange={setSoftResetOpen}>
                <AlertDialogTrigger asChild>
                  <Button className="w-full bg-[#ff9f0a] font-semibold text-black hover:bg-[#ffb340]">
                    Reset Settings
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent className="border-[#3a3a3c] bg-[#1c1c1e] text-white">
                  <AlertDialogHeader>
                    <AlertDialogTitle className="text-white">Soft Reset</AlertDialogTitle>
                    <AlertDialogDescription className="text-[#a1a1a6]">
                      This will reset all images and configuration settings to their default values.
                      Network settings (IP, DNS) will be preserved. The device will reboot.
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel className="border-[#3a3a3c] bg-transparent text-white hover:bg-[#2c2c2e]">
                      Cancel
                    </AlertDialogCancel>
                    <AlertDialogAction
                      onClick={() => softResetMutation.mutate()}
                      className="bg-[#ff9f0a] text-black hover:bg-[#ffb340]"
                    >
                      {softResetMutation.isPending ? 'Resetting...' : 'Confirm Reset'}
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            </SettingsCardContent>
          </SettingsCard>

          {/* Hard Reset */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(220,38,38,0.1)]">
                  <AlertTriangle className="size-5 text-[#dc2626]" />
                </div>
                <div>
                  <SettingsCardTitle>Hard factory reset</SettingsCardTitle>
                  <SettingsCardDescription>
                    Erase all data and reset to factory defaults
                  </SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent>
              <AlertDialog open={hardResetOpen} onOpenChange={setHardResetOpen}>
                <AlertDialogTrigger asChild>
                  <Button className="w-full bg-[#dc2626] font-semibold text-white hover:bg-[#ef4444]">
                    Factory Reset
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent className="border-[#3a3a3c] bg-[#1c1c1e] text-white">
                  <AlertDialogHeader>
                    <AlertDialogTitle className="text-white">Factory Reset</AlertDialogTitle>
                    <AlertDialogDescription className="text-[#a1a1a6]">
                      This is a destructive action. All settings, including network configuration
                      and users, will be permanently erased. The device will revert to its original
                      factory state.
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel className="border-[#3a3a3c] bg-transparent text-white hover:bg-[#2c2c2e]">
                      Cancel
                    </AlertDialogCancel>
                    <AlertDialogAction
                      onClick={() => hardResetMutation.mutate()}
                      className="bg-[#dc2626] text-white hover:bg-[#ef4444]"
                    >
                      {hardResetMutation.isPending ? 'Resetting...' : 'Erase Everything'}
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            </SettingsCardContent>
          </SettingsCard>

          {/* Reboot */}
          <SettingsCard className="lg:col-span-2">
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(255,255,255,0.1)]">
                  <Power className="size-5 text-white" />
                </div>
                <div>
                  <SettingsCardTitle>Reboot Device</SettingsCardTitle>
                  <SettingsCardDescription>Restart the system safely</SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent>
              <AlertDialog open={rebootOpen} onOpenChange={setRebootOpen}>
                <AlertDialogTrigger asChild>
                  <Button
                    variant="outline"
                    className="w-full border-[#3a3a3c] bg-transparent text-white hover:bg-[#2c2c2e]"
                  >
                    Restart System
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent className="border-[#3a3a3c] bg-[#1c1c1e] text-white">
                  <AlertDialogHeader>
                    <AlertDialogTitle className="text-white">Reboot Device?</AlertDialogTitle>
                    <AlertDialogDescription className="text-[#a1a1a6]">
                      The device will restart immediately. All active connections will be
                      terminated. This process typically takes about 60 seconds.
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel className="border-[#3a3a3c] bg-transparent text-white hover:bg-[#2c2c2e]">
                      Cancel
                    </AlertDialogCancel>
                    <AlertDialogAction
                      onClick={() => rebootMutation.mutate()}
                      className="bg-white text-black hover:bg-[#e5e5e5]"
                    >
                      {rebootMutation.isPending ? 'Rebooting...' : 'Confirm Reboot'}
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            </SettingsCardContent>
          </SettingsCard>
        </div>
      </div>
    </div>
  );
}
