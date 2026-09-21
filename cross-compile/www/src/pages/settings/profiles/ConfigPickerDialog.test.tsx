/**
 * ConfigPickerDialog Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { addConfiguration, getCompatibleConfigurations } from '@/services/profileService';
import { renderWithProviders } from '@/test/componentTestHelpers';

import { ConfigPickerDialog } from './ConfigPickerDialog';

vi.mock('@/services/profileService', () => ({
  getCompatibleConfigurations: vi.fn(),
  addConfiguration: vi.fn(),
}));

describe('ConfigPickerDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('lists candidates, attaches the selected one, and closes', async () => {
    vi.mocked(getCompatibleConfigurations).mockResolvedValue(['PTZConfig_0']);
    vi.mocked(addConfiguration).mockResolvedValue(undefined);

    const onAttached = vi.fn();
    const onOpenChange = vi.fn();

    renderWithProviders(
      <ConfigPickerDialog
        open
        onOpenChange={onOpenChange}
        title="PTZ"
        profileToken="P_1"
        configType="PTZ"
        onAttached={onAttached}
      />,
    );

    const option = await screen.findByTestId('config-picker-option-PTZConfig_0');
    expect(option).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(screen.getByTestId('config-picker-attach'));

    await waitFor(() => {
      expect(addConfiguration).toHaveBeenCalledWith('P_1', 'PTZ', 'PTZConfig_0');
      expect(onAttached).toHaveBeenCalled();
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });

  it('shows the empty state when the device advertises nothing', async () => {
    vi.mocked(getCompatibleConfigurations).mockResolvedValue([]);

    renderWithProviders(
      <ConfigPickerDialog
        open
        onOpenChange={() => {}}
        title="Metadata"
        profileToken="P_1"
        configType="Metadata"
      />,
    );

    expect(await screen.findByTestId('config-picker-empty')).toBeInTheDocument();
  });
});
