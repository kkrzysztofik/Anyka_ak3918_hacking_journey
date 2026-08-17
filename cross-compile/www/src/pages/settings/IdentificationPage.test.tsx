/**
 * IdentificationPage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getDeviceIdentification, getScopes, setScopes } from '@/services/deviceService';
import { getNetworkInterfaces } from '@/services/networkService';
import { MOCK_DATA, mockToast, renderWithProviders } from '@/test/componentTestHelpers';

import IdentificationPage from './IdentificationPage';

// Mock services
vi.mock('@/services/deviceService', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/deviceService')>();
  return {
    ...actual,
    getDeviceIdentification: vi.fn(),
    getScopes: vi.fn(),
    setScopes: vi.fn(),
  };
});

vi.mock('@/services/networkService', () => ({
  getNetworkInterfaces: vi.fn(),
}));

describe('IdentificationPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getDeviceIdentification).mockResolvedValue(MOCK_DATA.device);
    vi.mocked(getNetworkInterfaces).mockResolvedValue([]);
    vi.mocked(getScopes).mockResolvedValue([]);
  });

  it('should render identification form', async () => {
    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByTestId('identification-title')).toBeInTheDocument();
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
    vi.mocked(setScopes).mockResolvedValue(undefined);

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
      expect(setScopes).toHaveBeenCalledWith([
        'onvif://www.onvif.org/name/Updated%20Device',
        'onvif://www.onvif.org/location/Test%20Location',
      ]);
      expect(mockToast.success).toHaveBeenCalledWith('Device information saved');
    });
  });

  it('should show error toast when mutation fails', async () => {
    vi.mocked(setScopes).mockRejectedValue(new Error('Network error'));

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
        expect(setScopes).toHaveBeenCalled();
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
      expect(screen.getByTestId('identification-loading')).toBeInTheDocument();
    });
  });

  describe('scopes card', () => {
    const fixedScope = {
      scopeDef: 'Fixed' as const,
      scopeItem: 'onvif://www.onvif.org/type/video_encoder',
    };
    const customScope = {
      scopeDef: 'Configurable' as const,
      scopeItem: 'onvif://www.onvif.org/location/country/unknown',
    };

    beforeEach(() => {
      vi.mocked(getScopes).mockResolvedValue([fixedScope, customScope]);
    });

    it('should render fixed scopes as non-removable', async () => {
      renderWithProviders(<IdentificationPage />);

      await waitFor(() => {
        expect(
          screen.getByTestId(`identification-scope-row-${fixedScope.scopeItem}`),
        ).toBeInTheDocument();
      });

      expect(
        screen.getByTestId(`identification-scope-remove-${fixedScope.scopeItem}`),
      ).toBeDisabled();
      expect(
        screen.getByTestId(`identification-scope-remove-${customScope.scopeItem}`),
      ).not.toBeDisabled();
    });

    it('should add a scope row', async () => {
      const user = userEvent.setup();
      renderWithProviders(<IdentificationPage />);

      await waitFor(() => {
        expect(screen.getByTestId('identification-scope-add-input')).toBeInTheDocument();
      });

      const added = 'onvif://www.onvif.org/name/Extra';
      await user.type(screen.getByTestId('identification-scope-add-input'), added);
      await user.click(screen.getByTestId('identification-scope-add-button'));

      expect(screen.getByTestId(`identification-scope-row-${added}`)).toBeInTheDocument();
    });

    it('should remove a configurable scope row', async () => {
      const user = userEvent.setup();
      renderWithProviders(<IdentificationPage />);

      await waitFor(() => {
        expect(
          screen.getByTestId(`identification-scope-row-${customScope.scopeItem}`),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByTestId(`identification-scope-remove-${customScope.scopeItem}`));

      expect(
        screen.queryByTestId(`identification-scope-row-${customScope.scopeItem}`),
      ).not.toBeInTheDocument();
    });

    it('should send the full configurable list on save', async () => {
      vi.mocked(setScopes).mockResolvedValue(undefined);

      const user = userEvent.setup();
      renderWithProviders(<IdentificationPage />);

      await waitFor(() => {
        expect(screen.getByTestId('identification-device-name-input')).toHaveValue('Test Device');
        expect(
          screen.getByTestId(`identification-scope-row-${customScope.scopeItem}`),
        ).toBeInTheDocument();
      });

      const nameInput = screen.getByTestId('identification-device-name-input');
      await user.clear(nameInput);
      await user.type(nameInput, 'Updated Device');

      await user.click(screen.getByTestId('identification-save-button'));

      await waitFor(() => {
        expect(setScopes).toHaveBeenCalledWith(
          expect.arrayContaining([
            customScope.scopeItem,
            'onvif://www.onvif.org/name/Updated%20Device',
            'onvif://www.onvif.org/location/Test%20Location',
          ]),
        );
        expect(setScopes).toHaveBeenCalledWith(expect.not.arrayContaining([fixedScope.scopeItem]));
      });
    });
  });
});
