import { useState } from 'react';

import { useMutation, useQuery } from '@tanstack/react-query';
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
import { addConfiguration, getCompatibleConfigurations } from '@/services/profileService';

/**
 * Generic picker for attaching one of the device's configuration instances to a
 * profile. The candidate list is whatever the device advertises as compatible
 * with the profile; attaching is a profile-field update, not a config create.
 */
export function ConfigPickerDialog({
  open,
  onOpenChange,
  title,
  profileToken,
  configType,
  attachedToken,
  onAttached,
}: {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly title: string;
  readonly profileToken: string;
  readonly configType: string;
  readonly attachedToken?: string;
  readonly onAttached?: () => void;
}) {
  const [selected, setSelected] = useState<string | undefined>(attachedToken);

  const { data: candidates = [], isLoading } = useQuery<string[]>({
    queryKey: ['compatible-configurations', configType, profileToken],
    queryFn: () => getCompatibleConfigurations(profileToken, configType),
    enabled: open,
  });

  const effectiveSelected = selected ?? candidates[0];

  const attachMutation = useMutation({
    mutationFn: (token: string) => addConfiguration(profileToken, configType, token),
    onSuccess: () => {
      toast.success(`${title} attached`);
      onAttached?.();
      onOpenChange(false);
    },
    onError: (error) => {
      toast.error(`Failed to attach ${title}`, {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="border-[#3a3a3c] bg-[#1c1c1e] text-white sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle className="text-white" data-testid="config-picker-title">
            Attach {title}
          </DialogTitle>
          <DialogDescription className="text-[#a1a1a6]">
            Choose which {title.toLowerCase()} to use for this profile.
          </DialogDescription>
        </DialogHeader>

        <div className="py-2">
          {isLoading ? (
            <div className="py-6 text-center text-[#a1a1a6]">Loading…</div>
          ) : candidates.length === 0 ? (
            <div
              className="py-6 text-center text-[#a1a1a6] italic"
              data-testid="config-picker-empty"
            >
              No compatible configurations available.
            </div>
          ) : (
            <div className="space-y-2" role="radiogroup">
              {candidates.map((token) => (
                <label
                  key={token}
                  className={`flex cursor-pointer items-center gap-2 rounded-md border p-2 ${
                    effectiveSelected === token
                      ? 'border-[#0a84ff] bg-[rgba(10,132,255,0.1)]'
                      : 'border-[#3a3a3c]'
                  }`}
                >
                  <input
                    type="radio"
                    name={`config-picker-${configType}`}
                    value={token}
                    checked={effectiveSelected === token}
                    onChange={() => setSelected(token)}
                    className="accent-[#0a84ff]"
                    data-testid={`config-picker-option-${token}`}
                  />
                  <span className="font-mono text-[12px] text-white">{token}</span>
                </label>
              ))}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            className="border-[#3a3a3c] text-white hover:bg-[#2c2c2e]"
            data-testid="config-picker-cancel"
          >
            Cancel
          </Button>
          <Button
            onClick={() => effectiveSelected && attachMutation.mutate(effectiveSelected)}
            disabled={!effectiveSelected || attachMutation.isPending}
            className="bg-[#0a84ff] text-white hover:bg-[#0077ed]"
            data-testid="config-picker-attach"
          >
            {attachMutation.isPending ? 'Attaching…' : 'Attach'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
