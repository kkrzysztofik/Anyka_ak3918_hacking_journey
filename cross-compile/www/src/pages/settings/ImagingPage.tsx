/**
 * Imaging Page
 *
 * Configure camera image settings.
 */
import React, { useEffect, useState } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Camera, Contrast, Moon, Palette, RotateCcw, Save, ScanEye, Sun } from 'lucide-react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  SettingsCard,
  SettingsCardContent,
  SettingsCardDescription,
  SettingsCardHeader,
  SettingsCardTitle,
} from '@/components/ui/settings-card';
import { Slider } from '@/components/ui/slider';
import {
  type ImagingOptions,
  type ImagingSettings,
  getImagingOptions,
  getImagingSettings,
  setImagingSettings,
} from '@/services/imagingService';

export default function ImagingPage() {
  const queryClient = useQueryClient();

  // Fetch settings
  const { data: settings, isLoading } = useQuery<ImagingSettings>({
    queryKey: ['imagingSettings'],
    queryFn: () => getImagingSettings(),
  });

  // Fetch options (valid ranges)
  const { data: options } = useQuery<ImagingOptions>({
    queryKey: ['imagingOptions'],
    queryFn: () => getImagingOptions(),
  });

  // Local state for all form values
  const [localSettings, setLocalSettings] = useState<ImagingSettings>({
    brightness: 50,
    contrast: 50,
    saturation: 50,
    sharpness: 50,
    irCutFilter: 'AUTO',
    wideDynamicRange: {
      mode: 'OFF',
      level: 50,
    },
    backlightCompensation: {
      mode: 'OFF',
      level: 50,
    },
  });

  // Initialize form with fetched settings
  useEffect(() => {
    if (settings) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setLocalSettings((prev) => {
        // Only update if values actually changed to avoid unnecessary re-renders
        if (
          prev.brightness === settings.brightness &&
          prev.contrast === settings.contrast &&
          prev.saturation === settings.saturation &&
          prev.sharpness === settings.sharpness &&
          prev.irCutFilter === settings.irCutFilter &&
          JSON.stringify(prev.wideDynamicRange) === JSON.stringify(settings.wideDynamicRange) &&
          JSON.stringify(prev.backlightCompensation) ===
            JSON.stringify(settings.backlightCompensation)
        ) {
          return prev;
        }
        return {
          ...prev,
          ...settings,
          // Ensure defaults for optional fields
          wideDynamicRange: settings.wideDynamicRange ||
            prev.wideDynamicRange || {
              mode: 'OFF',
              level: 50,
            },
          backlightCompensation: settings.backlightCompensation ||
            prev.backlightCompensation || {
              mode: 'OFF',
              level: 50,
            },
        };
      });
    }
  }, [settings]);

  const mutation = useMutation({
    mutationFn: (newSettings: Partial<ImagingSettings>) => setImagingSettings(newSettings),
    onSuccess: () => {
      toast.success('Image settings saved');
      queryClient.invalidateQueries({ queryKey: ['imagingSettings'] });
    },
    onError: (error) => {
      toast.error('Failed to save image settings', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  const handleSave = () => {
    mutation.mutate({
      brightness: localSettings.brightness,
      contrast: localSettings.contrast,
      saturation: localSettings.saturation,
      sharpness: localSettings.sharpness,
      irCutFilter: localSettings.irCutFilter,
      wideDynamicRange: localSettings.wideDynamicRange,
      backlightCompensation: localSettings.backlightCompensation,
    });
  };

  const handleReset = () => {
    if (settings) {
      setLocalSettings((prev) => ({ ...prev, ...settings }));
      toast.info('Reset to current saved values');
    }
  };

  const updateSetting = <K extends keyof ImagingSettings>(key: K, value: ImagingSettings[K]) => {
    setLocalSettings((prev) => ({ ...prev, [key]: value }));
  };

  if (isLoading) {
    return <div className="text-white">Loading...</div>;
  }

  return (
    <div
      className="absolute inset-0 overflow-auto bg-[#0d0d0d] lg:inset-[0_0_0_356.84px]"
      data-name="Container"
    >
      <div className="max-w-[1200px] p-[16px] pb-[80px] md:p-[32px] md:pb-[48px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] flex flex-col justify-between gap-[16px] md:mb-[40px] md:flex-row md:items-center">
          <div>
            <h1 className="mb-[8px] text-[22px] text-white md:text-[28px]">Imaging</h1>
            <p className="text-[13px] text-[#a1a1a6] md:text-[14px]">
              Configure camera image quality, exposure, and day/night settings
            </p>
          </div>
          <div className="flex items-center gap-[12px]">
            <Button
              variant="outline"
              onClick={handleReset}
              className="h-[40px] rounded-[8px] border-[#3a3a3c] bg-transparent px-[20px] text-[#a1a1a6] hover:bg-[#1c1c1e] hover:text-white"
            >
              <RotateCcw className="mr-2 size-4" />
              Reset
            </Button>
            <Button
              onClick={handleSave}
              disabled={mutation.isPending}
              className="h-[40px] rounded-[8px] bg-[#dc2626] px-[20px] font-semibold text-white hover:bg-[#ef4444]"
            >
              <Save className="mr-2 size-4" />
              Save Changes
            </Button>
          </div>
        </div>

        <div className="grid grid-cols-1 gap-[24px] lg:grid-cols-2">
          {/* Color & Brightness */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(255,214,10,0.1)]">
                  <Sun className="size-5 text-[#ffd60a]" />
                </div>
                <div>
                  <SettingsCardTitle>Color & Brightness</SettingsCardTitle>
                  <SettingsCardDescription>Basic image adjustment</SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent className="space-y-[24px]">
              <div className="space-y-[12px]">
                <div className="flex items-center justify-between">
                  <Label className="text-[#e5e5e5]">Brightness</Label>
                  <span className="text-sm text-[#a1a1a6] tabular-nums">
                    {localSettings.brightness}%
                  </span>
                </div>
                <Slider
                  value={[localSettings.brightness]}
                  min={options?.brightness?.min ?? 0}
                  max={options?.brightness?.max ?? 100}
                  step={1}
                  onValueChange={([val]) => updateSetting('brightness', val)}
                  className="py-1"
                />
              </div>
              <div className="space-y-[12px]">
                <div className="flex items-center justify-between">
                  <Label className="text-[#e5e5e5]">Contrast</Label>
                  <span className="text-sm text-[#a1a1a6] tabular-nums">
                    {localSettings.contrast}%
                  </span>
                </div>
                <Slider
                  value={[localSettings.contrast]}
                  min={options?.contrast?.min ?? 0}
                  max={options?.contrast?.max ?? 100}
                  step={1}
                  onValueChange={([val]) => updateSetting('contrast', val)}
                  className="py-1"
                />
              </div>
              <div className="space-y-[12px]">
                <div className="flex items-center justify-between">
                  <Label className="text-[#e5e5e5]">Saturation</Label>
                  <span className="text-sm text-[#a1a1a6] tabular-nums">
                    {localSettings.saturation}%
                  </span>
                </div>
                <Slider
                  value={[localSettings.saturation]}
                  min={options?.saturation?.min ?? 0}
                  max={options?.saturation?.max ?? 100}
                  step={1}
                  onValueChange={([val]) => updateSetting('saturation', val)}
                  className="py-1"
                />
              </div>
            </SettingsCardContent>
          </SettingsCard>

          {/* Focus & Sharpness */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(10,132,255,0.1)]">
                  <ScanEye className="size-5 text-[#0a84ff]" />
                </div>
                <div>
                  <SettingsCardTitle>Focus & Sharpness</SettingsCardTitle>
                  <SettingsCardDescription>Lens focus and edge enhancement</SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent className="space-y-[24px]">
              <div className="space-y-[12px]">
                <div className="flex items-center justify-between">
                  <Label className="text-[#e5e5e5]">Sharpness</Label>
                  <span className="text-sm text-[#a1a1a6] tabular-nums">
                    {localSettings.sharpness}%
                  </span>
                </div>
                <Slider
                  value={[localSettings.sharpness]}
                  min={options?.sharpness?.min ?? 0}
                  max={options?.sharpness?.max ?? 100}
                  step={1}
                  onValueChange={([val]) => updateSetting('sharpness', val)}
                  className="py-1"
                />
              </div>
            </SettingsCardContent>
          </SettingsCard>

          {/* White Balance (STUB) */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(255,45,85,0.1)]">
                  <Palette className="size-5 text-[#ff2d55]" />
                </div>
                <div>
                  <SettingsCardTitle>White Balance</SettingsCardTitle>
                  <SettingsCardDescription>
                    Color temperature adjustment (Unavailable)
                  </SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent className="pointer-events-none space-y-[24px] opacity-60">
              <div className="space-y-[12px]">
                <Label className="text-[#e5e5e5]">Mode</Label>
                <select
                  value="auto"
                  disabled
                  className="h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white disabled:opacity-50"
                >
                  <option value="auto">Auto</option>
                </select>
              </div>
            </SettingsCardContent>
          </SettingsCard>

          {/* Exposure (STUB) */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(48,209,88,0.1)]">
                  <Camera className="size-5 text-[#30d158]" />
                </div>
                <div>
                  <SettingsCardTitle>Exposure Settings</SettingsCardTitle>
                  <SettingsCardDescription>
                    Shutter and gain control (Unavailable)
                  </SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent className="pointer-events-none space-y-[24px] opacity-60">
              <div className="space-y-[12px]">
                <Label className="text-[#e5e5e5]">Exposure Mode</Label>
                <select
                  value="auto"
                  disabled
                  className="h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white disabled:opacity-50"
                >
                  <option value="auto">Auto</option>
                </select>
              </div>
            </SettingsCardContent>
          </SettingsCard>

          {/* Infrared (IR Cut Filter) */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(191,90,242,0.1)]">
                  <Moon className="size-5 text-[#bf5af2]" />
                </div>
                <div>
                  <SettingsCardTitle>Infrared Settings</SettingsCardTitle>
                  <SettingsCardDescription>IR cut filter control</SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent className="space-y-[24px]">
              <div className="space-y-[12px]">
                <Label className="text-[#e5e5e5]">IR Cut Filter Mode</Label>
                <select
                  value={localSettings.irCutFilter || 'AUTO'}
                  onChange={(e) =>
                    updateSetting('irCutFilter', e.target.value as 'ON' | 'OFF' | 'AUTO')
                  }
                  className="h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white focus:border-transparent focus:ring-2 focus:ring-[#0a84ff] focus:outline-none"
                >
                  {options?.irCutFilterModes?.map((mode) => (
                    <option key={mode} value={mode}>
                      {mode === 'AUTO' ? 'Auto' : mode === 'ON' ? 'Day Mode' : 'Night Mode'}
                    </option>
                  )) || (
                    <>
                      <option value="AUTO">Auto</option>
                      <option value="ON">Day Mode</option>
                      <option value="OFF">Night Mode</option>
                    </>
                  )}
                </select>
              </div>
            </SettingsCardContent>
          </SettingsCard>

          {/* Backlight & WDR */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(255,159,10,0.1)]">
                  <Contrast className="size-5 text-[#ff9f0a]" />
                </div>
                <div>
                  <SettingsCardTitle>Backlight & WDR</SettingsCardTitle>
                  <SettingsCardDescription>
                    Wide Dynamic Range and backlight compensation
                  </SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent className="space-y-[24px]">
              {/* Wide Dynamic Range */}
              <div className="space-y-[12px]">
                <div className="flex items-center justify-between">
                  <Label className="text-[#e5e5e5]">WDR Mode</Label>
                </div>
                <select
                  value={localSettings.wideDynamicRange?.mode || 'OFF'}
                  onChange={(e) =>
                    updateSetting('wideDynamicRange', {
                      mode: e.target.value as 'ON' | 'OFF',
                      level: localSettings.wideDynamicRange?.level || 50,
                    })
                  }
                  className="h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white focus:border-transparent focus:ring-2 focus:ring-[#0a84ff] focus:outline-none"
                >
                  {options?.wideDynamicRange?.modes?.map((mode) => (
                    <option key={mode} value={mode}>
                      {mode}
                    </option>
                  )) || (
                    <>
                      <option value="OFF">Off</option>
                      <option value="ON">On</option>
                    </>
                  )}
                </select>
                {localSettings.wideDynamicRange?.mode === 'ON' && (
                  <div className="space-y-[12px]">
                    <div className="flex items-center justify-between">
                      <Label className="text-[#e5e5e5]">WDR Level</Label>
                      <span className="text-sm text-[#a1a1a6] tabular-nums">
                        {localSettings.wideDynamicRange.level}%
                      </span>
                    </div>
                    <Slider
                      value={[localSettings.wideDynamicRange.level]}
                      min={options?.wideDynamicRange?.level?.min ?? 0}
                      max={options?.wideDynamicRange?.level?.max ?? 100}
                      step={1}
                      onValueChange={([val]) =>
                        updateSetting('wideDynamicRange', {
                          mode: localSettings.wideDynamicRange?.mode || 'OFF',
                          level: val,
                        })
                      }
                      className="py-1"
                    />
                  </div>
                )}
              </div>

              {/* Backlight Compensation */}
              <div className="space-y-[12px]">
                <div className="flex items-center justify-between">
                  <Label className="text-[#e5e5e5]">Backlight Compensation Mode</Label>
                </div>
                <select
                  value={localSettings.backlightCompensation?.mode || 'OFF'}
                  onChange={(e) =>
                    updateSetting('backlightCompensation', {
                      mode: e.target.value as 'ON' | 'OFF',
                      level: localSettings.backlightCompensation?.level || 50,
                    })
                  }
                  className="h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white focus:border-transparent focus:ring-2 focus:ring-[#0a84ff] focus:outline-none"
                >
                  {options?.backlightCompensation?.modes?.map((mode) => (
                    <option key={mode} value={mode}>
                      {mode}
                    </option>
                  )) || (
                    <>
                      <option value="OFF">Off</option>
                      <option value="ON">On</option>
                    </>
                  )}
                </select>
                {localSettings.backlightCompensation?.mode === 'ON' && (
                  <div className="space-y-[12px]">
                    <div className="flex items-center justify-between">
                      <Label className="text-[#e5e5e5]">Backlight Level</Label>
                      <span className="text-sm text-[#a1a1a6] tabular-nums">
                        {localSettings.backlightCompensation.level}%
                      </span>
                    </div>
                    <Slider
                      value={[localSettings.backlightCompensation.level]}
                      min={options?.backlightCompensation?.level?.min ?? 0}
                      max={options?.backlightCompensation?.level?.max ?? 100}
                      step={1}
                      onValueChange={([val]) =>
                        updateSetting('backlightCompensation', {
                          mode: localSettings.backlightCompensation?.mode || 'OFF',
                          level: val,
                        })
                      }
                      className="py-1"
                    />
                  </div>
                )}
              </div>
            </SettingsCardContent>
          </SettingsCard>
        </div>
      </div>
    </div>
  );
}
