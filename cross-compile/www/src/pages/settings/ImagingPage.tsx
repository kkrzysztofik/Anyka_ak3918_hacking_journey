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
import { Switch } from '@/components/ui/switch';
import {
  type ImagingSettings,
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

  // Local state for all form values (including stubs)
  const [localSettings, setLocalSettings] = useState<
    ImagingSettings & {
      focusMode: string;
      whiteBalanceMode: string;
      whiteBalanceCr: number;
      whiteBalanceCb: number;
      wdrMode: string;
      wdrLevel: number;
      exposureMode: string;
      shutterLimit: string;
      exposureGain: number;
      irMode: string;
      irLevel: number;
    }
  >({
    brightness: 50,
    contrast: 50,
    saturation: 50,
    sharpness: 50,
    // Stubs
    focusMode: 'auto',
    whiteBalanceMode: 'auto',
    whiteBalanceCr: 50,
    whiteBalanceCb: 50,
    wdrMode: 'off',
    wdrLevel: 50,
    exposureMode: 'auto',
    shutterLimit: '1/30',
    exposureGain: 50,
    irMode: 'auto',
    irLevel: 100,
  });

  // Initialize form with fetched settings
  useEffect(() => {
    if (settings) {
      // Avoid setting state directly if only initial
      // or just use form.reset() style pattern if using forms
      // Here we are using local state for sliders, so we can just default to settings in useState if possible,
      // but since it's async, we need this effect.
      // The linter warning is about setLocalSettings causing re-render loop if settings changes often.
      // We can check if values are different before setting.
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setLocalSettings((prev) => {
        if (
          prev.brightness === settings.brightness &&
          prev.contrast === settings.contrast &&
          prev.saturation === settings.saturation &&
          prev.sharpness === settings.sharpness
        ) {
          return prev;
        }
        return { ...prev, ...settings };
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
    });
  };

  const handleReset = () => {
    if (settings) {
      setLocalSettings((prev) => ({ ...prev, ...settings }));
      toast.info('Reset to current saved values');
    }
  };

  const updateSetting = <K extends keyof typeof localSettings>(
    key: K,
    value: (typeof localSettings)[K],
  ) => {
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
                  min={0}
                  max={100}
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
                  min={0}
                  max={100}
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
                  min={0}
                  max={100}
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
                <Label className="text-[#e5e5e5]">Focus Mode</Label>
                <select
                  value={localSettings.focusMode}
                  onChange={(e) => updateSetting('focusMode', e.target.value)}
                  disabled
                  className="h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white focus:border-transparent focus:ring-2 focus:ring-[#0a84ff] focus:outline-none disabled:cursor-not-allowed disabled:opacity-50"
                >
                  <option value="auto">Auto Focus</option>
                  <option value="manual">Manual</option>
                </select>
              </div>
              <div className="space-y-[12px]">
                <div className="flex items-center justify-between">
                  <Label className="text-[#e5e5e5]">Sharpness</Label>
                  <span className="text-sm text-[#a1a1a6] tabular-nums">
                    {localSettings.sharpness}%
                  </span>
                </div>
                <Slider
                  value={[localSettings.sharpness]}
                  min={0}
                  max={100}
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

          {/* Infrared (STUB) */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(191,90,242,0.1)]">
                  <Moon className="size-5 text-[#bf5af2]" />
                </div>
                <div>
                  <SettingsCardTitle>Infrared Settings</SettingsCardTitle>
                  <SettingsCardDescription>
                    Night vision control (Unavailable)
                  </SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent className="pointer-events-none space-y-[24px] opacity-60">
              <div className="flex items-center justify-between">
                <div>
                  <Label className="block text-[#e5e5e5]">IR Mode</Label>
                  <span className="text-[13px] text-[#a1a1a6]">Auto-switch day/night</span>
                </div>
                <Switch checked disabled />
              </div>
            </SettingsCardContent>
          </SettingsCard>

          {/* Backlight & WDR (STUB) */}
          <SettingsCard>
            <SettingsCardHeader>
              <div className="flex items-center gap-[12px]">
                <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(255,159,10,0.1)]">
                  <Contrast className="size-5 text-[#ff9f0a]" />
                </div>
                <div>
                  <SettingsCardTitle>Backlight & WDR</SettingsCardTitle>
                  <SettingsCardDescription>
                    Wide Dynamic Range (Unavailable)
                  </SettingsCardDescription>
                </div>
              </div>
            </SettingsCardHeader>
            <SettingsCardContent className="pointer-events-none space-y-[24px] opacity-60">
              <div className="space-y-[12px]">
                <Label className="text-[#e5e5e5]">WDR Mode</Label>
                <select
                  value="off"
                  disabled
                  className="h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white disabled:opacity-50"
                >
                  <option value="off">Off</option>
                </select>
              </div>
            </SettingsCardContent>
          </SettingsCard>
        </div>
      </div>
    </div>
  );
}
