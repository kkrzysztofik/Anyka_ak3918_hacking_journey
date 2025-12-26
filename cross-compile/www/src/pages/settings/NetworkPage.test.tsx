/**
 * NetworkPage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { mockToast, renderWithProviders } from '@/test/componentTestHelpers';

import NetworkPage from './NetworkPage';

// Mock services
vi.mock('@/services/networkService', () => ({
  getNetworkConfig: vi.fn(),
  setNetworkInterface: vi.fn(),
  setDNS: vi.fn(),
}));

describe('NetworkPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const mockNetworkConfig = {
    interfaces: [
      {
        token: 'eth0',
        name: 'eth0',
        enabled: true,
        ipv4Enabled: true,
        dhcp: false,
        address: '192.168.1.100',
        prefixLength: 24,
        gateway: '192.168.1.1',
        hwAddress: '00:11:22:33:44:55',
      },
    ],
    dns: {
      fromDHCP: false,
      dnsServers: ['8.8.8.8', '8.8.4.4'],
      searchDomain: [],
    },
  };

  it('should render page with loading state', async () => {
    const { getNetworkConfig } = await import('@/services/networkService');
    vi.mocked(getNetworkConfig).mockImplementation(() => new Promise(() => {}));

    renderWithProviders(<NetworkPage />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('should render form with fetched network config', async () => {
    const { getNetworkConfig } = await import('@/services/networkService');
    vi.mocked(getNetworkConfig).mockResolvedValue(mockNetworkConfig);

    renderWithProviders(<NetworkPage />);

    await waitFor(() => {
      expect(screen.getByText('Network')).toBeInTheDocument();
    });

    expect(screen.getByTestId('network-ip-address-input')).toHaveValue('192.168.1.100');
    expect(screen.getByTestId('network-gateway-input')).toHaveValue('192.168.1.1');
  });

  it('should toggle DHCP and show/hide static IP fields', async () => {
    const { getNetworkConfig } = await import('@/services/networkService');
    vi.mocked(getNetworkConfig).mockResolvedValue(mockNetworkConfig);

    const user = userEvent.setup();
    renderWithProviders(<NetworkPage />);

    await waitFor(() => {
      expect(screen.getByText('Network')).toBeInTheDocument();
    });

    const dhcpSwitch = screen.getByTestId('network-dhcp-switch');
    expect(dhcpSwitch).toBeTruthy();

    // When DHCP is off, static fields should be visible
    const ipAddressInput = screen.getByTestId('network-ip-address-input');
    expect(ipAddressInput).toBeInTheDocument();

    // Toggle DHCP on
    await user.click(dhcpSwitch);
    await waitFor(() => {
      expect(dhcpSwitch).toBeChecked();
    });
  });

  it('should validate IP address format', async () => {
    const { getNetworkConfig } = await import('@/services/networkService');
    vi.mocked(getNetworkConfig).mockResolvedValue(mockNetworkConfig);

    const user = userEvent.setup();
    renderWithProviders(<NetworkPage />);

    await waitFor(() => {
      expect(screen.getByText('Network')).toBeInTheDocument();
    });

    const ipInput = screen.getByTestId('network-ip-address-input');
    await user.clear(ipInput);
    await user.type(ipInput, 'invalid-ip');

    const saveButton = screen.getByTestId('network-save-button');
    await user.click(saveButton);

    // Form validation should prevent submission
    await waitFor(() => {
      expect(screen.getByText(/invalid ip address/i)).toBeInTheDocument();
    });
  });

  it('should toggle DNS from DHCP', async () => {
    const { getNetworkConfig } = await import('@/services/networkService');
    vi.mocked(getNetworkConfig).mockResolvedValue(mockNetworkConfig);

    const user = userEvent.setup();
    renderWithProviders(<NetworkPage />);

    await waitFor(() => {
      expect(screen.getByText('Network')).toBeInTheDocument();
    });

    const dnsFromDHCPSwitch = screen.getByTestId('network-dns-from-dhcp-switch');
    expect(dnsFromDHCPSwitch).toBeTruthy();

    await user.click(dnsFromDHCPSwitch);
    await waitFor(() => {
      expect(dnsFromDHCPSwitch).toBeChecked();
    });
  });

  it('should show DNS input fields when DNS from DHCP is off', async () => {
    const { getNetworkConfig } = await import('@/services/networkService');
    vi.mocked(getNetworkConfig).mockResolvedValue(mockNetworkConfig);

    renderWithProviders(<NetworkPage />);

    await waitFor(() => {
      expect(screen.getByTestId('network-primary-dns-input')).toHaveValue('8.8.8.8');
      expect(screen.getByTestId('network-secondary-dns-input')).toHaveValue('8.8.4.4');
    });
  });

  it('should show confirmation dialog on form submission', async () => {
    const { getNetworkConfig } = await import('@/services/networkService');
    vi.mocked(getNetworkConfig).mockResolvedValue(mockNetworkConfig);

    const user = userEvent.setup();
    renderWithProviders(<NetworkPage />);

    await waitFor(() => {
      expect(screen.getByText('Network')).toBeInTheDocument();
    });

    // Make form dirty by changing a value
    const ipInput = screen.getByTestId('network-ip-address-input');
    await user.clear(ipInput);
    await user.type(ipInput, '192.168.1.200');

    const saveButton = screen.getByTestId('network-save-button');
    expect(saveButton).toBeTruthy();
    await user.click(saveButton);

    await waitFor(
      () => {
        expect(screen.getByTestId('network-confirm-dialog')).toBeInTheDocument();
        expect(screen.getByText('Save Network Settings?')).toBeInTheDocument();
      },
      { timeout: 3000 },
    );
  });

  it('should call mutation on confirmation', async () => {
    const { getNetworkConfig, setNetworkInterface, setDNS } =
      await import('@/services/networkService');
    vi.mocked(getNetworkConfig).mockResolvedValue(mockNetworkConfig);
    vi.mocked(setNetworkInterface).mockResolvedValue(undefined);
    vi.mocked(setDNS).mockResolvedValue(undefined);

    const user = userEvent.setup();
    renderWithProviders(<NetworkPage />);

    await waitFor(() => {
      expect(screen.getByText('Network')).toBeInTheDocument();
    });

    // Make form dirty
    const ipInput = screen.getByTestId('network-ip-address-input');
    await user.clear(ipInput);
    await user.type(ipInput, '192.168.1.200');

    const saveButton = screen.getByTestId('network-save-button');
    expect(saveButton).toBeTruthy();
    await user.click(saveButton);

    await waitFor(
      () => {
        expect(screen.getByText('Save Network Settings?')).toBeInTheDocument();
      },
      { timeout: 3000 },
    );

    const confirmButton = screen.getByTestId('network-confirm-save-button');
    expect(confirmButton).toBeTruthy();
    await user.click(confirmButton);

    await waitFor(() => {
      expect(setNetworkInterface).toHaveBeenCalled();
      expect(setDNS).toHaveBeenCalled();
      expect(mockToast.success).toHaveBeenCalledWith('Network settings saved', {
        description: 'The device may lose connectivity if IP settings changed.',
      });
    });
  });

  it('should show error toast when mutation fails', async () => {
    const { getNetworkConfig, setNetworkInterface } = await import('@/services/networkService');
    vi.mocked(getNetworkConfig).mockResolvedValue(mockNetworkConfig);
    vi.mocked(setNetworkInterface).mockRejectedValue(new Error('Network error'));

    const user = userEvent.setup();
    renderWithProviders(<NetworkPage />);

    await waitFor(() => {
      expect(screen.getByText('Network')).toBeInTheDocument();
    });

    // Make form dirty
    const ipInput = screen.getByTestId('network-ip-address-input');
    await user.clear(ipInput);
    await user.type(ipInput, '192.168.1.200');

    const saveButton = screen.getByTestId('network-save-button');
    await user.click(saveButton);

    await waitFor(
      () => {
        expect(screen.getByText('Save Network Settings?')).toBeInTheDocument();
      },
      { timeout: 3000 },
    );

    const confirmButton = screen.getByTestId('network-confirm-save-button');
    await user.click(confirmButton);

    await waitFor(() => {
      expect(mockToast.error).toHaveBeenCalledWith('Failed to save settings', {
        description: 'Network error',
      });
    });
  });

  it('should reset form when reset button is clicked', async () => {
    const { getNetworkConfig } = await import('@/services/networkService');
    vi.mocked(getNetworkConfig).mockResolvedValue(mockNetworkConfig);

    const user = userEvent.setup();
    renderWithProviders(<NetworkPage />);

    await waitFor(() => {
      expect(screen.getByText('Network')).toBeInTheDocument();
    });

    const ipInput = screen.getByTestId('network-ip-address-input');
    await user.clear(ipInput);
    await user.type(ipInput, '192.168.1.200');

    const resetButton = screen.getByTestId('network-reset-button');
    await user.click(resetButton);

    await waitFor(() => {
      expect(mockToast.info).toHaveBeenCalledWith('Form reset to current values');
      expect(ipInput).toHaveValue('192.168.1.100');
    });
  });

  it('should render connection status card', async () => {
    const { getNetworkConfig } = await import('@/services/networkService');
    vi.mocked(getNetworkConfig).mockResolvedValue(mockNetworkConfig);

    renderWithProviders(<NetworkPage />);

    await waitFor(() => {
      expect(screen.getByText('MAC Address')).toBeInTheDocument();
      expect(screen.getByText('00:11:22:33:44:55')).toBeInTheDocument();
    });
  });
});
