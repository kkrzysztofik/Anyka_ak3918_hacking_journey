/**
 * PresetDialog Component Tests
 */
import React from 'react';

import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getPtzStatus, setPreset } from '@/services/ptzService';
import type { PTZStatus } from '@/services/ptzService';
import { renderWithProviders } from '@/test/componentTestHelpers';

import { PresetDialog } from './PresetDialog';

vi.mock('@/services/ptzService', () => ({
  getPtzStatus: vi.fn().mockResolvedValue({ pan: 0.42, tilt: -0.085, zoom: 0 }),
  setPreset: vi.fn().mockResolvedValue('Preset1'),
}));

vi.mock('sonner', () => ({
  toast: { error: vi.fn(), success: vi.fn(), info: vi.fn() },
}));

const PRESET = { token: 'Preset1', name: 'Front Door' };

describe('PresetDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getPtzStatus).mockResolvedValue({ pan: 0.42, tilt: -0.085, zoom: 0 });
    vi.mocked(setPreset).mockResolvedValue('Preset1');
  });

  it('warns that updating also re-saves the position', async () => {
    renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset={PRESET} onClose={vi.fn()} />,
    );

    await waitFor(() => {
      expect(screen.getByTestId('preset-dialog-position-warning')).toHaveTextContent(
        /also re-saves the position as 0\.42 \/ -0\.09/i,
      );
    });
  });

  it('hides the cached position while a reopened dialog refetches', async () => {
    // Reopening serves the previous read straight from cache. Showing it would name a
    // stale aim point for the position SetPreset is about to overwrite.
    const { unmount, queryClient } = renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset={PRESET} onClose={vi.fn()} />,
    );
    await waitFor(() => {
      expect(screen.getByTestId('preset-dialog-position')).toHaveTextContent('0.42 / -0.09');
    });
    unmount();

    let releaseSecondRead: (status: PTZStatus) => void = () => {};
    vi.mocked(getPtzStatus).mockReturnValue(
      new Promise<PTZStatus>((resolve) => {
        releaseSecondRead = resolve;
      }),
    );
    renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset={PRESET} onClose={vi.fn()} />,
      { queryClient },
    );

    expect(screen.getByTestId('preset-dialog-position')).toHaveTextContent('Position: —');
    expect(screen.getByTestId('preset-dialog-position-warning')).toHaveTextContent(
      /re-saves the current position/i,
    );

    releaseSecondRead({ pan: -1.5, tilt: 0.25, zoom: 0 });
    await waitFor(() => {
      expect(screen.getByTestId('preset-dialog-position')).toHaveTextContent('-1.50 / 0.25');
    });
  });

  it('does not warn about a position rewrite when creating', () => {
    renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset="new" onClose={vi.fn()} />,
    );

    expect(screen.queryByTestId('preset-dialog-position-warning')).not.toBeInTheDocument();
  });

  it('seeds the name field from the preset being edited', () => {
    renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset={PRESET} onClose={vi.fn()} />,
    );

    expect(screen.getByTestId('preset-dialog-name-input')).toHaveValue('Front Door');
  });

  it('suggests a non-colliding name after presets were deleted', () => {
    // Two presets remain but their tokens run to Preset5, so the old length+1 rule
    // would suggest "Preset 3" — a name already taken.
    renderWithProviders(
      <PresetDialog
        profileToken="ProfileToken1"
        preset="new"
        existing={[
          { token: 'Preset5', name: 'Gate' },
          { token: 'Preset2', name: 'Yard' },
        ]}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByTestId('preset-dialog-name-input')).toHaveValue('Preset 6');
  });

  it('still saves when the position read fails', async () => {
    // The position is advisory. Refusing to save because an advisory read failed
    // would be worse than the problem it reports.
    vi.mocked(getPtzStatus).mockRejectedValue(new Error('SOAP timeout'));
    const user = userEvent.setup();
    renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset={PRESET} onClose={vi.fn()} />,
    );

    await waitFor(() => {
      expect(screen.getByTestId('preset-dialog-position-warning')).toHaveTextContent(
        /re-saves the current position/i,
      );
    });
    await user.click(screen.getByTestId('preset-dialog-submit'));

    await waitFor(() => {
      expect(setPreset).toHaveBeenCalledWith('ProfileToken1', 'Front Door', 'Preset1');
    });
  });

  it('omits the token when creating so the backend mints one', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset="new" onClose={vi.fn()} />,
    );

    await user.click(screen.getByTestId('preset-dialog-submit'));

    await waitFor(() => {
      expect(setPreset).toHaveBeenCalledWith('ProfileToken1', 'Preset 1', undefined);
    });
  });

  it('closes without saving when cancelled', async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset={PRESET} onClose={onClose} />,
    );

    await user.click(screen.getByTestId('preset-dialog-cancel'));

    expect(onClose).toHaveBeenCalled();
    expect(setPreset).not.toHaveBeenCalled();
  });

  it('disables submit for a whitespace-only name', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset={PRESET} onClose={vi.fn()} />,
    );

    await user.clear(screen.getByTestId('preset-dialog-name-input'));
    await user.type(screen.getByTestId('preset-dialog-name-input'), '   ');

    expect(screen.getByTestId('preset-dialog-submit')).toBeDisabled();
  });

  it('trims the name before saving', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset={PRESET} onClose={vi.fn()} />,
    );

    await user.clear(screen.getByTestId('preset-dialog-name-input'));
    await user.type(screen.getByTestId('preset-dialog-name-input'), '  Driveway  ');
    await user.click(screen.getByTestId('preset-dialog-submit'));

    await waitFor(() => {
      expect(setPreset).toHaveBeenCalledWith('ProfileToken1', 'Driveway', 'Preset1');
    });
  });

  it('stays open on a failed save so typing is not lost', async () => {
    vi.mocked(setPreset).mockRejectedValue(new Error('device busy'));
    const onClose = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset={PRESET} onClose={onClose} />,
    );

    await user.click(screen.getByTestId('preset-dialog-submit'));

    await waitFor(() => expect(setPreset).toHaveBeenCalled());
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByTestId('preset-dialog')).toBeInTheDocument();
  });
});

describe('PresetDialog name suggestion', () => {
  it('starts at 1 with no presets', () => {
    renderWithProviders(
      <PresetDialog profileToken="ProfileToken1" preset="new" existing={[]} onClose={vi.fn()} />,
    );

    expect(screen.getByTestId('preset-dialog-name-input')).toHaveValue('Preset 1');
  });

  it('falls back to counting when no token looks like PresetN', () => {
    renderWithProviders(
      <PresetDialog
        profileToken="ProfileToken1"
        preset="new"
        existing={[{ token: 'custom-abc', name: 'Gate' }]}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByTestId('preset-dialog-name-input')).toHaveValue('Preset 2');
  });
});
