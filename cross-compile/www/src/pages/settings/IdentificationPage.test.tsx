/**
 * IdentificationPage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  getDeviceIdentification,
  getDiscoveryMode,
  getHostname,
  getScopes,
  setDiscoveryMode,
  setHostname,
  setScopes,
} from '@/services/deviceService';
import { getDiagnostics } from '@/services/diagnosticsService';
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
    getHostname: vi.fn(),
    getDiscoveryMode: vi.fn(),
    setScopes: vi.fn(),
    setHostname: vi.fn(),
    setDiscoveryMode: vi.fn(),
  };
});

vi.mock('@/services/networkService', () => ({
  getNetworkInterfaces: vi.fn(),
}));

vi.mock('@/services/diagnosticsService', () => ({
  getDiagnostics: vi.fn(),
}));

const MOCK_DIAGNOSTICS = {
  status: 'healthy',
  firmware_version: 'test',
  uptime: { process_s: 100, system_s: 7200 },
  cpu_percent: null,
  memory: null,
  storage: null,
  network: null,
  stream_frame_age_ms: null,
  components: [],
  degraded_services: [],
  vision: null,
  wifi: {
    interface: 'wlan0',
    connected: true,
    ssid: 'kmk',
    frequency_mhz: 2437,
    channel: 6,
    security: 'WPA2',
    signal_dbm: -52,
    link_quality: '66/70',
  },
};

describe('IdentificationPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getDeviceIdentification).mockResolvedValue(MOCK_DATA.device);
    vi.mocked(getNetworkInterfaces).mockResolvedValue([]);
    vi.mocked(getDiagnostics).mockResolvedValue(MOCK_DIAGNOSTICS);
    vi.mocked(getScopes).mockResolvedValue([]);
    vi.mocked(getHostname).mockResolvedValue('ipcam');
    vi.mocked(getDiscoveryMode).mockResolvedValue('Discoverable');
  });

  it('should render identification form', async () => {
    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByTestId('identification-title')).toBeInTheDocument();
    });
  });

  it('should show diagnostics-backed status card values', async () => {
    vi.mocked(getNetworkInterfaces).mockResolvedValue([
      {
        token: 'wlan0',
        enabled: true,
        name: 'wlan0',
        hwAddress: 'C0:4B:24:DA:4D:EB',
        linkSpeedMbps: null,
        ipv4Enabled: true,
        dhcp: true,
        address: '192.168.2.198',
        prefixLength: 24,
        gateway: '',
      },
    ]);

    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByTestId('identification-status-health')).toHaveTextContent('Healthy');
    });
    expect(screen.getByTestId('identification-status-uptime')).toHaveTextContent('2h 0m');
    expect(screen.getByTestId('identification-status-mac')).toHaveTextContent('C0:4B:24:DA:4D:EB');
    expect(screen.getByTestId('identification-status-quality')).toHaveTextContent('66/70');
    expect(screen.getByTestId('identification-status-channel')).toHaveTextContent('6');
    expect(screen.getByTestId('identification-status-security')).toHaveTextContent('WPA2');
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

  it('should preserve unsaved edits when query data refetches', async () => {
    const user = userEvent.setup();
    const { queryClient } = renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByTestId('identification-device-name-input')).toHaveValue('Test Device');
    });

    const nameInput = screen.getByTestId('identification-device-name-input');
    await user.clear(nameInput);
    await user.type(nameInput, 'Unsaved Name');

    await queryClient.invalidateQueries({ queryKey: ['deviceInformation'] });

    await waitFor(() => {
      expect(nameInput).toHaveValue('Unsaved Name');
    });
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
      expect(
        screen.getByTestId(`identification-scope-remove-${customScope.scopeItem}`),
      ).toHaveAttribute('aria-label', `Remove scope ${customScope.scopeItem}`);
      expect(screen.getByTestId('identification-scope-add-input')).toHaveAttribute(
        'aria-label',
        'New ONVIF scope',
      );
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

  describe('discovery and hostname', () => {
    it('should apply discovery mode immediately without saving', async () => {
      vi.mocked(setDiscoveryMode).mockResolvedValue(undefined);

      const user = userEvent.setup();
      renderWithProviders(<IdentificationPage />);

      await waitFor(() => {
        expect(screen.getByTestId('identification-discovery-switch')).toBeInTheDocument();
      });

      await user.click(screen.getByTestId('identification-discovery-switch'));

      await waitFor(() => {
        expect(setDiscoveryMode).toHaveBeenCalledWith('NonDiscoverable');
      });
      expect(setScopes).not.toHaveBeenCalled();
    });

    it('should omit hostname from save when it is unchanged', async () => {
      vi.mocked(setScopes).mockResolvedValue(undefined);
      vi.mocked(setHostname).mockResolvedValue(undefined);

      const user = userEvent.setup();
      renderWithProviders(<IdentificationPage />);

      await waitFor(() => {
        expect(screen.getByTestId('identification-hostname-input')).toHaveValue('ipcam');
      });

      const nameInput = screen.getByTestId('identification-device-name-input');
      await user.clear(nameInput);
      await user.type(nameInput, 'Updated Device');
      await user.click(screen.getByTestId('identification-save-button'));

      await waitFor(() => {
        expect(setScopes).toHaveBeenCalled();
      });
      expect(setHostname).not.toHaveBeenCalled();
    });

    it('should discard hostname edits back to the loaded value', async () => {
      const user = userEvent.setup();
      renderWithProviders(<IdentificationPage />);

      await waitFor(() => {
        expect(screen.getByTestId('identification-hostname-input')).toHaveValue('ipcam');
      });

      const hostnameInput = screen.getByTestId('identification-hostname-input');
      await user.clear(hostnameInput);
      await user.type(hostnameInput, 'front-door');
      await user.click(screen.getByTestId('identification-reset-button'));

      await waitFor(() => {
        expect(hostnameInput).toHaveValue('ipcam');
      });
      expect(screen.getByTestId('identification-reset-button')).toHaveTextContent(
        'Discard Changes',
      );
    });
  });
});
