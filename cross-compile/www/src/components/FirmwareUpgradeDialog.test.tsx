/**
 * FirmwareUpgradeDialog — design tests 2–8
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError } from '@/services/api';
import type { Diagnostics } from '@/services/diagnosticsService';
import { getDiagnostics, uploadFirmware } from '@/services/diagnosticsService';
import { createControllablePromise, renderWithProviders } from '@/test/componentTestHelpers';

import { FirmwareUpgradeDialog } from './FirmwareUpgradeDialog';

vi.mock('@/services/diagnosticsService', () => ({
  getDiagnostics: vi.fn(),
  uploadFirmware: vi.fn(),
}));

const PREVIOUS = 'v1.0.0';

function makeDiagnostics(firmware_version: string): Diagnostics {
  return {
    status: 'ok',
    firmware_version,
    uptime: { process_s: 1, system_s: 1 },
    cpu_percent: null,
    memory: null,
    storage: null,
    network: null,
    stream_frame_age_ms: null,
    components: [],
    degraded_services: [],
    vision: null,
  };
}

function renderDialog(
  overrides: Partial<{
    open: boolean;
    onOpenChange: (open: boolean) => void;
    previousVersion: string | null;
    onFinished: () => void;
  }> = {},
) {
  const props = {
    open: true,
    onOpenChange: vi.fn(),
    previousVersion: PREVIOUS,
    onFinished: vi.fn(),
    ...overrides,
  };
  renderWithProviders(<FirmwareUpgradeDialog {...props} />);
  return props;
}

async function selectTarAndContinue(
  user: ReturnType<typeof userEvent.setup>,
  file = new File(['tar'], 'bundle.tar', { type: 'application/x-tar' }),
) {
  await user.upload(screen.getByTestId('firmware-upgrade-input'), file);
  await user.click(screen.getByTestId('firmware-upgrade-continue-button'));
}

describe('FirmwareUpgradeDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(uploadFirmware).mockResolvedValue(undefined);
    vi.mocked(getDiagnostics).mockRejectedValue(new Error('camera down'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('should disable continue without a valid .tar file', async () => {
    const user = userEvent.setup();
    renderDialog();

    const continueButton = screen.getByTestId('firmware-upgrade-continue-button');
    expect(continueButton).toBeDisabled();

    await user.upload(screen.getByTestId('firmware-upgrade-input'), new File(['x'], 'bundle.bin'));
    expect(continueButton).toBeDisabled();

    const oversized = new File(['x'], 'big.tar');
    Object.defineProperty(oversized, 'size', { value: 64 * 1024 * 1024 + 1 });
    await user.upload(screen.getByTestId('firmware-upgrade-input'), oversized);
    expect(continueButton).toBeDisabled();

    await user.upload(
      screen.getByTestId('firmware-upgrade-input'),
      new File(['tar'], 'bundle.tar'),
    );
    expect(continueButton).toBeEnabled();
  });

  it('should show AlertDialog confirm before calling uploadFirmware', async () => {
    const user = userEvent.setup();
    const upload = createControllablePromise<void>();
    vi.mocked(uploadFirmware).mockReturnValue(upload.promise);

    renderDialog();
    await selectTarAndContinue(user);

    expect(screen.getByTestId('firmware-upgrade-confirm-button')).toBeInTheDocument();
    expect(uploadFirmware).not.toHaveBeenCalled();

    await user.click(screen.getByTestId('firmware-upgrade-confirm-button'));

    await waitFor(() => {
      expect(uploadFirmware).toHaveBeenCalledTimes(1);
    });
  });

  it('should update progress UI from onProgress callback', async () => {
    const user = userEvent.setup();
    const upload = createControllablePromise<void>();
    vi.mocked(uploadFirmware).mockImplementation(async (_file, options) => {
      options?.onProgress?.({ loaded: 5, total: 10 });
      return upload.promise;
    });

    renderDialog();
    await selectTarAndContinue(user);
    await user.click(screen.getByTestId('firmware-upgrade-confirm-button'));

    const progress = await screen.findByTestId('firmware-upgrade-progress');
    expect(progress).toHaveAttribute('value', '5');
    expect(progress).toHaveAttribute('max', '10');
  });

  it('should enter waiting after a 202 upload', async () => {
    const user = userEvent.setup();
    vi.mocked(uploadFirmware).mockResolvedValue(undefined);

    renderDialog();
    await selectTarAndContinue(user);
    await user.click(screen.getByTestId('firmware-upgrade-confirm-button'));

    expect(await screen.findByTestId('firmware-upgrade-waiting')).toBeInTheDocument();
    await waitFor(() => {
      expect(getDiagnostics).toHaveBeenCalled();
    });
  });

  it('should ignore pre-reboot diagnostics until a down→up reconnect', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });

    vi.mocked(uploadFirmware).mockResolvedValue(undefined);
    let poll = 0;
    vi.mocked(getDiagnostics).mockImplementation(async () => {
      poll += 1;
      if (poll === 1) return makeDiagnostics(PREVIOUS);
      if (poll === 2) throw new Error('camera down');
      return makeDiagnostics('v2.0.0');
    });

    renderDialog();
    await selectTarAndContinue(user);
    await user.click(screen.getByTestId('firmware-upgrade-confirm-button'));

    expect(await screen.findByTestId('firmware-upgrade-waiting')).toBeInTheDocument();

    await waitFor(() => {
      expect(poll).toBeGreaterThanOrEqual(1);
    });
    // First success is still the old version before reboot — must not exit early
    expect(screen.queryByTestId('firmware-upgrade-result-message')).not.toBeInTheDocument();
    expect(screen.getByTestId('firmware-upgrade-waiting')).toBeInTheDocument();

    await waitFor(
      () => {
        expect(screen.getByTestId('firmware-upgrade-result-message').textContent).toMatch(
          /committed/i,
        );
      },
      { timeout: 10_000 },
    );
  });

  it('should show committed copy when firmware_version changes', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });

    vi.mocked(uploadFirmware).mockResolvedValue(undefined);
    vi.mocked(getDiagnostics)
      .mockRejectedValueOnce(new Error('camera down'))
      .mockResolvedValue(makeDiagnostics('v2.0.0'));

    renderDialog();
    await selectTarAndContinue(user);
    await user.click(screen.getByTestId('firmware-upgrade-confirm-button'));

    await waitFor(
      () => {
        expect(screen.getByTestId('firmware-upgrade-result-message').textContent).toMatch(
          /committed/i,
        );
      },
      { timeout: 5000 },
    );
  });

  it('should show reverted copy when firmware_version is unchanged', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });

    vi.mocked(uploadFirmware).mockResolvedValue(undefined);
    vi.mocked(getDiagnostics)
      .mockRejectedValueOnce(new Error('camera down'))
      .mockResolvedValue(makeDiagnostics(PREVIOUS));

    renderDialog();
    await selectTarAndContinue(user);
    await user.click(screen.getByTestId('firmware-upgrade-confirm-button'));

    await waitFor(
      () => {
        expect(screen.getByTestId('firmware-upgrade-result-message').textContent).toMatch(
          /probably reverted/i,
        );
      },
      { timeout: 5000 },
    );
  });

  it('should show upload error on uploading step and allow retry', async () => {
    const user = userEvent.setup();
    vi.mocked(uploadFirmware).mockRejectedValueOnce(
      new ApiError('Firmware upload failed with status 500', 500, ''),
    );

    renderDialog();
    await selectTarAndContinue(user);
    await user.click(screen.getByTestId('firmware-upgrade-confirm-button'));

    const error = await screen.findByTestId('firmware-upgrade-error');
    expect(error.textContent).toMatch(/500|failed/i);
    // Stay put on uploading — do not collapse back to select
    expect(screen.getByTestId('firmware-upgrade-progress')).toBeInTheDocument();
    expect(screen.queryByTestId('firmware-upgrade-input')).not.toBeInTheDocument();
    expect(screen.queryByTestId('firmware-upgrade-waiting')).not.toBeInTheDocument();

    vi.mocked(uploadFirmware).mockResolvedValueOnce(undefined);
    await user.click(screen.getByTestId('firmware-upgrade-continue-button'));
    await user.click(screen.getByTestId('firmware-upgrade-confirm-button'));

    await waitFor(() => {
      expect(uploadFirmware).toHaveBeenCalledTimes(2);
    });
  });
});
