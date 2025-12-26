/**
 * IdentificationPage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getDeviceIdentification, setDeviceInformation } from '@/services/deviceService';
import { getNetworkInterfaces } from '@/services/networkService';
import { MOCK_DATA, mockToast, renderWithProviders } from '@/test/componentTestHelpers';

import IdentificationPage from './IdentificationPage';

// Mock services
vi.mock('@/services/deviceService', () => ({
  getDeviceIdentification: vi.fn(),
  setDeviceInformation: vi.fn(),
}));

vi.mock('@/services/networkService', () => ({
  getNetworkInterfaces: vi.fn(),
}));

describe('IdentificationPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getDeviceIdentification).mockResolvedValue(MOCK_DATA.device);
    vi.mocked(getNetworkInterfaces).mockResolvedValue([]);
  });

  it('should render identification form', async () => {
    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByText(/device identification/i)).toBeInTheDocument();
    });
  });

  it('should display device information when loaded', async () => {
    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByTestId('identification-device-name-input')).toHaveValue('Test Device');
      expect(screen.getByTestId('identification-device-location-input')).toHaveValue(
        'Test Location',
      );
    });
  });

  it('should allow editing device name', async () => {
    const user = userEvent.setup();
    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByTestId('identification-device-name-input')).toHaveValue('Test Device');
    });

    const nameInput = screen.getByTestId('identification-device-name-input');
    await user.clear(nameInput);
    await user.type(nameInput, 'Updated Device Name');

    expect(nameInput).toHaveValue('Updated Device Name');
  });

  it('should allow editing device location', async () => {
    const user = userEvent.setup();
    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByTestId('identification-device-location-input')).toHaveValue(
        'Test Location',
      );
    });

    const locationInput = screen.getByTestId('identification-device-location-input');
    await user.clear(locationInput);
    await user.type(locationInput, 'Updated Location');

    expect(locationInput).toHaveValue('Updated Location');
  });

  it('should submit form with valid data', async () => {
    vi.mocked(setDeviceInformation).mockResolvedValue(undefined);

    const user = userEvent.setup();
    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByTestId('identification-device-name-input')).toHaveValue('Test Device');
    });

    const nameInput = screen.getByTestId('identification-device-name-input');
    await user.clear(nameInput);
    await user.type(nameInput, 'Updated Device');

    const submitButton = screen.getByTestId('identification-save-button');
    await user.click(submitButton);

    await waitFor(() => {
      expect(setDeviceInformation).toHaveBeenCalled();
      expect(mockToast.success).toHaveBeenCalledWith('Device information saved');
    });
  });

  it('should show error toast when mutation fails', async () => {
    vi.mocked(setDeviceInformation).mockRejectedValue(new Error('Network error'));

    const user = userEvent.setup();
    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByTestId('identification-device-name-input')).toHaveValue('Test Device');
    });

    const nameInput = screen.getByTestId('identification-device-name-input');
    await user.clear(nameInput);
    await user.type(nameInput, 'Updated Device');

    const submitButton = screen.getByTestId('identification-save-button');
    await user.click(submitButton);

    await waitFor(
      () => {
        expect(setDeviceInformation).toHaveBeenCalled();
        expect(mockToast.error).toHaveBeenCalledWith('Failed to save device information', {
          description: 'Network error',
        });
      },
      { timeout: 3000 },
    );
  });

  it('should reset form when handleReset is called', async () => {
    const user = userEvent.setup();
    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByTestId('identification-device-name-input')).toHaveValue('Test Device');
    });

    const nameInput = screen.getByTestId('identification-device-name-input');
    await user.clear(nameInput);
    await user.type(nameInput, 'Changed Name');

    const resetButton = screen.getByTestId('identification-reset-button');
    await user.click(resetButton);

    await waitFor(() => {
      expect(mockToast.info).toHaveBeenCalledWith('Form reset to current device values');
      expect(nameInput).toHaveValue('Test Device');
    });
  });

  it('should show loading state when device info is loading', async () => {
    vi.mocked(getDeviceIdentification).mockImplementation(
      () => new Promise(() => {}), // Never resolves
    );

    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByText('Loading...')).toBeInTheDocument();
    });
  });
});
