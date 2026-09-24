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
  type AudioEncoderConfiguration,
  type AudioEncoderConfigurationOptions,
  getAudioEncoderConfiguration,
  getAudioEncoderConfigurationOptions,
  setAudioEncoderConfiguration,
} from '@/services/profileService';

const SELECT_CLASS =
  'h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white focus:border-[#0a84ff] focus:outline-none';

export function AudioEncoderDialog({
  encoderToken,
  onClose,
}: {
  readonly encoderToken: string;
  readonly onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const [config, setConfig] = React.useState<AudioEncoderConfiguration | null>(null);
  const [options, setOptions] = React.useState<AudioEncoderConfigurationOptions | null>(null);
  const [isLoading, setIsLoading] = React.useState(true);
  const controlId = React.useId();

  // Stabilize onClose so it doesn't cause re-fetches when the parent re-renders
  const onCloseRef = React.useRef(onClose);
  React.useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  // Fetch the configuration and the options the device advertises for it. There
  // is nothing to edit without both, so a missing either closes the dialog.
  React.useEffect(() => {
    const controller = new AbortController();
    const loadData = async () => {
      try {
        const [encoderConfig, encoderOptions] = await Promise.all([
          getAudioEncoderConfiguration(encoderToken),
          getAudioEncoderConfigurationOptions(encoderToken),
        ]);
        if (controller.signal.aborted) return;
        if (!encoderConfig || !encoderOptions || encoderOptions.options.length === 0) {
          toast.error('Audio encoder configuration not found');
          onCloseRef.current();
          return;
        }
        setConfig(encoderConfig);
        setOptions(encoderOptions);
      } catch (error) {
        if (controller.signal.aborted) return;
        toast.error('Failed to load audio encoder configuration', {
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
    // The device replaces the stored configuration wholesale, so the whole
    // edited copy goes back — not just the changed field.
    mutationFn: (updatedConfig: AudioEncoderConfiguration) =>
      setAudioEncoderConfiguration(updatedConfig),
    onSuccess: () => {
      toast.success('Audio encoder configuration updated');
      queryClient.invalidateQueries({ queryKey: ['profiles'] });
      onClose();
    },
    onError: (error) => {
      toast.error('Failed to update audio encoder configuration', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  if (isLoading || !config || !options) {
    return (
      <Dialog open onOpenChange={onClose}>
        <DialogContent
          className="border-[#3a3a3c] bg-[#1c1c1e] text-white sm:max-w-[425px]"
          data-testid="audio-encoder-edit-dialog"
        >
          <DialogHeader>
            <DialogTitle className="sr-only">Loading Audio Encoder Configuration</DialogTitle>
            <DialogDescription className="sr-only">
              Loading audio encoder settings dialog content
            </DialogDescription>
          </DialogHeader>
          <div
            className="py-8 text-center text-[#a1a1a6]"
            data-testid="audio-encoder-edit-dialog-loading"
          >
            Loading...
          </div>
        </DialogContent>
      </Dialog>
    );
  }

  const selected =
    options.options.find((o) => o.encoding === config.encoding) ?? options.options[0];

  // A different encoding brings its own bitrate/sample-rate lists, so land on
  // the first value the new encoding advertises instead of keeping a stale one.
  const changeEncoding = (encoding: string) => {
    const next = options.options.find((o) => o.encoding === encoding);
    setConfig({
      ...config,
      encoding,
      bitrate: next?.bitrates[0] ?? config.bitrate,
      sampleRate: next?.sampleRates[0] ?? config.sampleRate,
    });
  };

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent
        className="border-[#3a3a3c] bg-[#1c1c1e] text-white sm:max-w-[425px]"
        data-testid="audio-encoder-edit-dialog"
      >
        <DialogHeader>
          <DialogTitle className="text-white" data-testid="audio-encoder-edit-dialog-title">
            Edit Audio Encoder Configuration
          </DialogTitle>
          <DialogDescription className="text-[#a1a1a6]">
            Configure audio encoding settings for this profile
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label className="text-[#e5e5e5]" htmlFor={`${controlId}-encoding`}>
              Encoding
            </Label>
            <select
              id={`${controlId}-encoding`}
              value={config.encoding}
              onChange={(e) => changeEncoding(e.target.value)}
              className={SELECT_CLASS}
              data-testid="audio-encoder-encoding-select"
            >
              {options.options.map((o) => (
                <option key={o.encoding} value={o.encoding}>
                  {o.encoding}
                </option>
              ))}
            </select>
          </div>

          <div className="space-y-2">
            <Label className="text-[#e5e5e5]" htmlFor={`${controlId}-bitrate`}>
              Bitrate
            </Label>
            <select
              id={`${controlId}-bitrate`}
              value={config.bitrate}
              onChange={(e) => setConfig({ ...config, bitrate: Number(e.target.value) })}
              className={SELECT_CLASS}
              data-testid="audio-encoder-bitrate-select"
            >
              {(selected?.bitrates ?? []).map((bitrate) => (
                <option key={bitrate} value={bitrate}>
                  {bitrate} kbps
                </option>
              ))}
            </select>
          </div>

          <div className="space-y-2">
            <Label className="text-[#e5e5e5]" htmlFor={`${controlId}-sample-rate`}>
              Sample Rate
            </Label>
            <select
              id={`${controlId}-sample-rate`}
              value={config.sampleRate}
              onChange={(e) => setConfig({ ...config, sampleRate: Number(e.target.value) })}
              className={SELECT_CLASS}
              data-testid="audio-encoder-samplerate-select"
            >
              {(selected?.sampleRates ?? []).map((sampleRate) => (
                <option key={sampleRate} value={sampleRate}>
                  {sampleRate} kHz
                </option>
              ))}
            </select>
          </div>
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={onClose}
            className="border-[#3a3a3c] text-white hover:bg-[#2c2c2e]"
            data-testid="audio-encoder-edit-dialog-cancel"
          >
            Cancel
          </Button>
          <Button
            type="button"
            onClick={() => updateMutation.mutate(config)}
            disabled={updateMutation.isPending}
            className="bg-[#0a84ff] text-white hover:bg-[#0077ed]"
            data-testid="audio-encoder-edit-dialog-save"
          >
            {updateMutation.isPending ? 'Saving...' : 'Save Changes'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
