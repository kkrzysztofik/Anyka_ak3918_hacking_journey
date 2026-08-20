/**
 * Preset Dialog
 *
 * Creates or updates a PTZ preset. One dialog serves both, because ONVIF only has one
 * operation: SetPreset carrying an existing token replaces the stored *position* as well
 * as the name (onvif-rust/src/onvif/ptz/store.rs), and 24.12 defines no RenamePreset.
 *
 * So there is deliberately no rename-only path. A "rename" button would silently re-aim
 * the preset at wherever the camera happens to be pointing — data loss disguised as a
 * text edit. Instead the dialog states the position it is about to write before the user
 * commits, which is the whole reason it reads GetStatus at all.
 */
import { useState } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
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
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { getPtzStatus, setPreset } from '@/services/ptzService';
import type { PTZPreset } from '@/services/ptzService';

/** Tokens the backend mints look like `Preset7`. */
const PRESET_TOKEN_PATTERN = /^Preset(\d+)$/;

/**
 * Suggest a name for a new preset that will not collide with an existing one.
 *
 * Counting the presets (`length + 1`) collides after any delete — two presets left over
 * from Preset1/Preset2/Preset5 would suggest "Preset 3", which already exists. Take one
 * past the highest token suffix instead.
 */
function suggestPresetName(existing: readonly PTZPreset[]): string {
  const highest = existing.reduce((max, preset) => {
    const suffix = PRESET_TOKEN_PATTERN.exec(preset.token)?.[1];
    return suffix ? Math.max(max, Number(suffix)) : max;
  }, 0);
  return `Preset ${highest > 0 ? highest + 1 : existing.length + 1}`;
}

interface PresetDialogProps {
  profileToken: string;
  /** The preset being edited, or `'new'` to create one. */
  preset: PTZPreset | 'new';
  /** Presets already on the device, used only to suggest a non-colliding name. */
  existing?: readonly PTZPreset[];
  onClose: () => void;
}

export function PresetDialog({
  profileToken,
  preset,
  existing = [],
  onClose,
}: Readonly<PresetDialogProps>) {
  const isEditing = preset !== 'new';
  const [name, setName] = useState(() => (isEditing ? preset.name : suggestPresetName(existing)));
  const queryClient = useQueryClient();

  // Read once per open, not polled: this number is only visible in this modal, and a
  // refetchInterval would cost a SOAP round trip per tick for it.
  const {
    data: position,
    isFetching,
    isFetchedAfterMount,
  } = useQuery({
    queryKey: ['ptzStatus', profileToken],
    queryFn: () => getPtzStatus(profileToken),
    enabled: !!profileToken,
    staleTime: 0,
    retry: false,
  });

  const saveMutation = useMutation({
    mutationFn: (presetName: string) =>
      setPreset(profileToken, presetName, isEditing ? preset.token : undefined),
    onSuccess: () => {
      toast.success('Preset saved');
      queryClient.invalidateQueries({ queryKey: ['ptzPresets', profileToken] });
      onClose();
    },
    onError: (error) => {
      // Deliberately stays open: closing here would throw away what the user typed.
      toast.error('Save preset failed', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  const trimmed = name.trim();
  // Reopening the dialog serves the previous position straight from cache while the
  // refetch is still in flight. Showing that number would disclose where the camera
  // *was*, then save where it *is* — the exact silent re-aim this dialog exists to
  // prevent. Only trust coordinates fetched during this mount and settled.
  const fresh = isFetchedAfterMount && !isFetching ? position : undefined;
  const positionText = fresh ? `${fresh.pan.toFixed(2)} / ${fresh.tilt.toFixed(2)}` : '—';

  return (
    <Dialog open onOpenChange={(next) => !next && onClose()}>
      <DialogContent
        className="border-border bg-card text-foreground sm:max-w-[425px]"
        data-testid="preset-dialog"
      >
        <DialogHeader>
          <DialogTitle>{isEditing ? 'Update preset' : 'Save preset'}</DialogTitle>
          <DialogDescription className="text-muted-foreground">
            {isEditing
              ? 'Save the current camera position over this preset.'
              : 'Save where the camera is pointing right now.'}
          </DialogDescription>
        </DialogHeader>

        <form
          className="space-y-4"
          onSubmit={(e) => {
            e.preventDefault();
            if (trimmed) saveMutation.mutate(trimmed);
          }}
        >
          <div className="space-y-2">
            <Label htmlFor="preset-dialog-name" className="text-muted-foreground">
              Preset name
            </Label>
            <Input
              id="preset-dialog-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="border-border bg-transparent"
              data-testid="preset-dialog-name-input"
            />
          </div>

          <p className="text-muted-foreground text-xs" data-testid="preset-dialog-position">
            Position: {positionText} (estimated)
          </p>

          {isEditing && (
            <p className="text-xs text-yellow-500" data-testid="preset-dialog-position-warning">
              {fresh
                ? `This also re-saves the position as ${positionText} (estimated).`
                : 'This also re-saves the current position.'}
            </p>
          )}

          <DialogFooter className="mt-4">
            <Button
              type="button"
              variant="outline"
              className="border-border bg-transparent"
              onClick={onClose}
              data-testid="preset-dialog-cancel"
            >
              Cancel
            </Button>
            <Button
              type="submit"
              className="bg-accent-red text-white hover:bg-red-700"
              disabled={!trimmed || saveMutation.isPending}
              data-testid="preset-dialog-submit"
            >
              {isEditing ? 'Update' : 'Save'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
