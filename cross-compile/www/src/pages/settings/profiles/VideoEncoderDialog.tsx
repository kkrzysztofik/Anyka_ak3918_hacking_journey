import * as React from 'react';

import { useMutation, useQueryClient } from '@tanstack/react-query';
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
import { Label } from '@/components/ui/label';
import {
  type VideoEncoderConfiguration,
  type VideoEncoderConfigurationOptions,
  getVideoEncoderConfiguration,
  getVideoEncoderConfigurationOptions,
  setVideoEncoderConfiguration,
} from '@/services/profileService';

export function VideoEncoderDialog({
  encoderToken,
  onClose,
}: {
  readonly encoderToken: string;
  readonly onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const [config, setConfig] = React.useState<VideoEncoderConfiguration | null>(null);
  const [options, setOptions] = React.useState<VideoEncoderConfigurationOptions | null>(null);
  const [isLoading, setIsLoading] = React.useState(true);

  // Stabilize onClose so it doesn't cause re-fetches when the parent re-renders
  const onCloseRef = React.useRef(onClose);
  React.useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  // Fetch encoder configuration and options
  React.useEffect(() => {
    const controller = new AbortController();
    const loadData = async () => {
      try {
        const [encoderConfig, encoderOptions] = await Promise.all([
          getVideoEncoderConfiguration(encoderToken),
          getVideoEncoderConfigurationOptions(encoderToken),
        ]);
        if (controller.signal.aborted) return;
        if (encoderConfig) {
          setConfig(encoderConfig);
        }
        setOptions(encoderOptions);
      } catch (error) {
        if (controller.signal.aborted) return;
        toast.error('Failed to load encoder configuration', {
          description: error instanceof Error ? error.message : 'An error occurred',
        });
        onCloseRef.current();
      } finally {
        if (!controller.signal.aborted) {
          setIsLoading(false);
        }
      }
    };
    loadData();
    return () => controller.abort();
  }, [encoderToken]);

  const updateMutation = useMutation({
    mutationFn: (updatedConfig: VideoEncoderConfiguration) =>
      setVideoEncoderConfiguration(updatedConfig, false),
    onSuccess: () => {
      toast.success('Video encoder configuration updated');
      queryClient.invalidateQueries({ queryKey: ['profiles'] });
      onClose();
    },
    onError: (error) => {
      toast.error('Failed to update encoder configuration', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  const handleSave = () => {
    if (config) {
      updateMutation.mutate(config);
    }
  };

  if (isLoading || !config || !options) {
    return (
      <Dialog open onOpenChange={onClose}>
        <DialogContent
          className="border-[#3a3a3c] bg-[#1c1c1e] text-white sm:max-w-[600px]"
          data-testid="video-encoder-edit-dialog"
        >
          <DialogHeader>
            <DialogTitle className="sr-only">Loading Video Encoder Configuration</DialogTitle>
            <DialogDescription className="sr-only">
              Loading encoder settings dialog content
            </DialogDescription>
          </DialogHeader>
          <div
            className="py-8 text-center text-[#a1a1a6]"
            data-testid="video-encoder-edit-dialog-loading"
          >
            Loading...
          </div>
        </DialogContent>
      </Dialog>
    );
  }

  const h264Options = options.h264;
  const availableResolutions = h264Options?.resolutionsAvailable || [];

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent
        className="border-[#3a3a3c] bg-[#1c1c1e] text-white sm:max-w-[600px]"
        data-testid="video-encoder-edit-dialog"
      >
        <DialogHeader>
          <DialogTitle className="text-white" data-testid="video-encoder-edit-dialog-title">
            Edit Video Encoder Configuration
          </DialogTitle>
          <DialogDescription className="text-[#a1a1a6]">
            Configure video encoding settings for this profile
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Resolution */}
          <div className="space-y-2">
            <Label className="text-[#e5e5e5]">Resolution</Label>
            <select
              value={`${config.resolution.width}x${config.resolution.height}`}
              onChange={(e) => {
                const [width, height] = e.target.value.split('x').map(Number);
                setConfig({ ...config, resolution: { width, height } });
              }}
              className="h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white focus:border-[#0a84ff] focus:outline-none"
              data-testid="video-encoder-resolution-select"
            >
              {availableResolutions.map((res) => (
                <option key={`${res.width}x${res.height}`} value={`${res.width}x${res.height}`}>
                  {res.width} × {res.height}
                </option>
              ))}
            </select>
          </div>

          {/* Quality */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-[#e5e5e5]">Quality</Label>
              <span className="text-sm text-[#a1a1a6] tabular-nums">{config.quality}</span>
            </div>
            <input
              type="range"
              min={options.qualityRange.min}
              max={options.qualityRange.max}
              value={config.quality}
              onChange={(e) => setConfig({ ...config, quality: Number(e.target.value) })}
              className="w-full"
              data-testid="video-encoder-quality-slider"
            />
          </div>

          {/* Frame Rate */}
          {config.rateControl && h264Options && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label className="text-[#e5e5e5]">Frame Rate Limit</Label>
                <span className="text-sm text-[#a1a1a6] tabular-nums">
                  {config.rateControl.frameRateLimit} fps
                </span>
              </div>
              <input
                type="range"
                min={h264Options.frameRateRange.min}
                max={h264Options.frameRateRange.max}
                value={config.rateControl.frameRateLimit}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    rateControl: {
                      ...config.rateControl!,
                      frameRateLimit: Number(e.target.value),
                    },
                  })
                }
                className="w-full"
                data-testid="video-encoder-framerate-slider"
              />
            </div>
          )}

          {/* Bitrate */}
          {config.rateControl && h264Options && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label className="text-[#e5e5e5]">Bitrate Limit</Label>
                <span className="text-sm text-[#a1a1a6] tabular-nums">
                  {config.rateControl.bitrateLimit} kbps
                </span>
              </div>
              <input
                type="range"
                min={h264Options.bitrateRange.min}
                max={h264Options.bitrateRange.max}
                value={config.rateControl.bitrateLimit}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    rateControl: {
                      ...config.rateControl!,
                      bitrateLimit: Number(e.target.value),
                    },
                  })
                }
                className="w-full"
                data-testid="video-encoder-bitrate-slider"
              />
            </div>
          )}

          {/* H.264 Profile */}
          {config.h264 && h264Options && (
            <div className="space-y-2">
              <Label className="text-[#e5e5e5]">H.264 Profile</Label>
              <select
                value={config.h264.h264Profile}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    h264: { ...config.h264!, h264Profile: e.target.value },
                  })
                }
                className="h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white focus:border-[#0a84ff] focus:outline-none"
                data-testid="video-encoder-h264-profile-select"
              >
                {h264Options.h264ProfilesSupported.map((profile) => (
                  <option key={profile} value={profile}>
                    {profile}
                  </option>
                ))}
              </select>
            </div>
          )}

          {/* GOP Length */}
          {config.h264 && h264Options && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label className="text-[#e5e5e5]">GOP Length</Label>
                <span className="text-sm text-[#a1a1a6] tabular-nums">{config.h264.govLength}</span>
              </div>
              <input
                type="range"
                min={h264Options.govLengthRange.min}
                max={h264Options.govLengthRange.max}
                value={config.h264.govLength}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    h264: { ...config.h264!, govLength: Number(e.target.value) },
                  })
                }
                className="w-full"
                data-testid="video-encoder-gop-slider"
              />
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={onClose}
            className="border-[#3a3a3c] text-white hover:bg-[#2c2c2e]"
            data-testid="video-encoder-edit-dialog-cancel"
          >
            Cancel
          </Button>
          <Button
            type="button"
            onClick={handleSave}
            disabled={updateMutation.isPending}
            className="bg-[#0a84ff] text-white hover:bg-[#0077ed]"
            data-testid="video-encoder-edit-dialog-save"
          >
            {updateMutation.isPending ? 'Saving...' : 'Save Changes'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
