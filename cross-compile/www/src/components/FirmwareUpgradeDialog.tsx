import { useCallback, useEffect, useId, useRef, useState } from 'react';

import { Upload } from 'lucide-react';

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ApiError } from '@/services/api';
import { getDiagnostics, uploadFirmware } from '@/services/diagnosticsService';

const MAX_BYTES = 64 * 1024 * 1024;
const POLL_INTERVAL_MS = 2000;
const POLL_TIMEOUT_MS = 5 * 60 * 1000;

type Step = 'select' | 'uploading' | 'waiting' | 'result';

export interface FirmwareUpgradeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  previousVersion: string | null;
  onFinished: () => void;
}

function isValidTar(file: File | null): file is File {
  return (
    !!file && file.name.toLowerCase().endsWith('.tar') && file.size > 0 && file.size <= MAX_BYTES
  );
}

function sleep(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal.aborted) {
      reject(new DOMException('Aborted', 'AbortError'));
      return;
    }
    const onAbort = () => {
      window.clearTimeout(id);
      reject(new DOMException('Aborted', 'AbortError'));
    };
    const id = window.setTimeout(() => {
      signal.removeEventListener('abort', onAbort);
      resolve();
    }, ms);
    signal.addEventListener('abort', onAbort);
  });
}

function isAbortError(err: unknown): boolean {
  return err instanceof DOMException
    ? err.name === 'AbortError'
    : err instanceof Error && err.name === 'AbortError';
}

export function FirmwareUpgradeDialog({
  open,
  onOpenChange,
  previousVersion,
  onFinished,
}: Readonly<FirmwareUpgradeDialogProps>) {
  const inputRef = useRef<HTMLInputElement>(null);
  const progressLabelId = useId();
  const [file, setFile] = useState<File | null>(null);
  const [step, setStep] = useState<Step>('select');
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [progress, setProgress] = useState<{ loaded: number; total: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [resultMessage, setResultMessage] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  const reset = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
    setFile(null);
    setStep('select');
    setConfirmOpen(false);
    setProgress(null);
    setError(null);
    setResultMessage(null);
    setDragging(false);
    if (inputRef.current) inputRef.current.value = '';
  }, []);

  useEffect(() => {
    return () => {
      abortRef.current?.abort();
    };
  }, []);

  const assignFile = useCallback((next: File | null) => {
    setFile(next);
    setError(null);
    if (next) {
      if (!next.name.toLowerCase().endsWith('.tar')) {
        setError('Bundle must be a .tar file.');
      } else if (next.size === 0) {
        setError('Bundle is empty.');
      } else if (next.size > MAX_BYTES) {
        setError(`Bundle is too large (max 64 MB).`);
      }
    }
  }, []);

  const handleBrowse = useCallback(() => {
    inputRef.current?.click();
  }, []);

  const handleClose = useCallback(() => {
    if (step === 'result') onFinished();
    reset();
    onOpenChange(false);
  }, [onFinished, onOpenChange, reset, step]);

  const pollUntilBack = useCallback(
    async (signal: AbortSignal) => {
      const deadline = Date.now() + POLL_TIMEOUT_MS;
      // ponytail: down→up edge approximates reconnect; trial-status API if false reverted reports appear.
      let sawDown = false;
      while (Date.now() < deadline) {
        if (signal.aborted) throw new DOMException('Aborted', 'AbortError');
        try {
          const diagnostics = await getDiagnostics(signal);
          if (sawDown) {
            const next = diagnostics.firmware_version;
            if (next !== previousVersion) {
              setResultMessage(`Upgrade committed. Firmware version is now ${next}.`);
            } else {
              setResultMessage(`Upgrade probably reverted. Firmware version is still ${next}.`);
            }
            setStep('result');
            return;
          }
          // Still reachable with a pre-reboot snapshot — keep polling.
        } catch (err) {
          if (isAbortError(err)) throw err;
          sawDown = true;
        }
        await sleep(POLL_INTERVAL_MS, signal);
      }
      setResultMessage(
        sawDown
          ? 'Camera still unreachable. Refresh later.'
          : 'Timed out waiting for reboot. The camera stayed reachable — check whether the update applied.',
      );
      setStep('result');
    },
    [previousVersion],
  );

  const startUpload = useCallback(async () => {
    if (!isValidTar(file)) return;

    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;

    setConfirmOpen(false);
    setStep('uploading');
    setProgress({ loaded: 0, total: file.size || 1 });
    setError(null);

    try {
      await uploadFirmware(file, {
        signal: controller.signal,
        onProgress: (p) => setProgress(p),
      });
      setStep('waiting');
      await pollUntilBack(controller.signal);
    } catch (err) {
      if (isAbortError(err) || controller.signal.aborted) {
        setStep('select');
        setProgress(null);
        setError(null);
        return;
      }
      // Stay on uploading so the error is visible next to progress; retry allowed.
      // Prefer the backend's own detail (checksum, schema, authorization) over
      // the generic HTTP-status message when the response body carries one.
      let message = 'Upload failed';
      if (err instanceof ApiError && err.data.trim().length > 0) {
        message = err.data;
      } else if (err instanceof Error) {
        message = err.message;
      }
      setError(message);
    }
  }, [file, pollUntilBack]);

  const onDrop = useCallback(
    (event: React.DragEvent<HTMLDivElement>) => {
      event.preventDefault();
      setDragging(false);
      const dropped = event.dataTransfer.files?.[0] ?? null;
      assignFile(dropped);
    },
    [assignFile],
  );

  const valid = isValidTar(file);
  const dismissLocked = step === 'waiting' || (step === 'uploading' && !error);

  return (
    <>
      <Dialog
        open={open}
        onOpenChange={(next) => {
          if (!next && dismissLocked) return;
          if (!next) handleClose();
          else onOpenChange(next);
        }}
      >
        <DialogContent
          className={`bg-card border-border text-foreground sm:max-w-[480px]${dismissLocked ? '[&_[data-testid=dialog-close]]:hidden' : ''}`}
          data-testid="firmware-upgrade-dialog"
          onInteractOutside={(event) => {
            if (dismissLocked) event.preventDefault();
          }}
          onEscapeKeyDown={(event) => {
            if (dismissLocked) event.preventDefault();
          }}
        >
          <DialogHeader>
            <div className="mb-2 flex items-center gap-3">
              <div className="flex size-10 items-center justify-center rounded-lg bg-green-500/10">
                <Upload className="h-5 w-5 text-green-500" />
              </div>
              <div>
                <DialogTitle className="text-xl">Firmware Upgrade</DialogTitle>
                <DialogDescription className="text-muted-foreground">
                  Select a .tar bundle, confirm, then wait for the camera to return.
                </DialogDescription>
              </div>
            </div>
          </DialogHeader>

          {step === 'select' && (
            <div className="space-y-4 py-2">
              <div
                className={`border-border rounded-lg border border-dashed p-6 text-center ${
                  dragging ? 'bg-muted/40' : 'bg-background'
                }`}
                onDragOver={(e) => {
                  e.preventDefault();
                  setDragging(true);
                }}
                onDragLeave={() => setDragging(false)}
                onDrop={onDrop}
              >
                <p className="text-muted-foreground mb-3 text-sm">
                  Drop a .tar bundle here, or browse (max 64&nbsp;MB).
                </p>
                <input
                  ref={inputRef}
                  type="file"
                  accept=".tar"
                  className="hidden"
                  data-testid="firmware-upgrade-input"
                  onChange={(e) => assignFile(e.target.files?.[0] ?? null)}
                />
                <Button type="button" variant="outline" size="sm" onClick={handleBrowse}>
                  {file?.name ?? 'Choose bundle…'}
                </Button>
              </div>
              {error && (
                <p
                  className="text-destructive text-sm"
                  data-testid="firmware-upgrade-error"
                  aria-live="polite"
                >
                  {error}
                </p>
              )}
            </div>
          )}

          {step === 'uploading' && progress && (
            <div className="space-y-3 py-4">
              <p className="text-muted-foreground text-sm" id={progressLabelId}>
                {error ? 'Upload failed' : 'Uploading…'}
              </p>
              <progress
                className="h-2 w-full"
                data-testid="firmware-upgrade-progress"
                aria-labelledby={progressLabelId}
                value={progress.loaded}
                max={progress.total || 1}
              />
              {error && (
                <p
                  className="text-destructive text-sm"
                  data-testid="firmware-upgrade-error"
                  aria-live="assertive"
                >
                  {error}
                </p>
              )}
            </div>
          )}

          {step === 'waiting' && (
            <p
              className="text-muted-foreground py-4 text-sm"
              data-testid="firmware-upgrade-waiting"
              aria-live="polite"
            >
              Camera is rebooting. Waiting for it to come back…
            </p>
          )}

          {step === 'result' && resultMessage && (
            <p
              className="py-4 text-sm"
              data-testid="firmware-upgrade-result-message"
              aria-live="polite"
            >
              {resultMessage}
            </p>
          )}

          <DialogFooter className="pt-2">
            {step === 'select' && (
              <>
                <Button
                  type="button"
                  variant="ghost"
                  onClick={handleClose}
                  data-testid="firmware-upgrade-close-button"
                >
                  Close
                </Button>
                <Button
                  type="button"
                  disabled={!valid}
                  onClick={() => setConfirmOpen(true)}
                  data-testid="firmware-upgrade-continue-button"
                >
                  Continue
                </Button>
              </>
            )}
            {step === 'result' && (
              <Button
                type="button"
                onClick={handleClose}
                data-testid="firmware-upgrade-close-button"
              >
                Close
              </Button>
            )}
            {step === 'uploading' && error && (
              <>
                <Button
                  type="button"
                  variant="ghost"
                  onClick={handleClose}
                  data-testid="firmware-upgrade-close-button"
                >
                  Cancel
                </Button>
                <Button
                  type="button"
                  disabled={!valid}
                  onClick={() => setConfirmOpen(true)}
                  data-testid="firmware-upgrade-continue-button"
                >
                  Retry
                </Button>
              </>
            )}
            {((step === 'uploading' && !error) || step === 'waiting') && (
              <Button type="button" variant="ghost" disabled>
                Please wait…
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
        <AlertDialogContent className="bg-card border-border text-foreground">
          <AlertDialogHeader>
            <AlertDialogTitle>Confirm firmware upgrade?</AlertDialogTitle>
            <AlertDialogDescription>
              The camera will reboot for about 2 minutes. If services do not bind, it will
              auto-rollback to the previous slot.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              data-testid="firmware-upgrade-confirm-button"
              onClick={(e) => {
                e.preventDefault();
                void startUpload();
              }}
            >
              Confirm
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
