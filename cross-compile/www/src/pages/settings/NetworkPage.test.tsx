/**
 * NetworkPage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getNetworkConfig, setDNS, setNetworkInterface } from '@/services/networkService';
import {
  MOCK_DATA,
  fillFormField,
  makeFormDirty,
  mockToast,
  renderWithProviders,
  waitForPageLoad,
} from '@/test/componentTestHelpers';

import NetworkPage from './NetworkPage';

// Mock services
vi.mock('@/services/networkService', () => ({
  getNetworkConfig: vi.fn(),
  setNetworkInterface: vi.fn(),
  setDNS: vi.fn(),
}));

describe('NetworkPage', () => {
  const renderNetworkPage = async () => {
    const result = renderWithProviders(<NetworkPage />);
    await waitForPageLoad('network-title');
    return result;
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getNetworkConfig).mockResolvedValue(MOCK_DATA.network);
  });

  it('should render page with loading state', async () => {
    vi.mocked(getNetworkConfig).mockImplementation(() => new Promise(() => {}));

    renderWithProviders(<NetworkPage />);
    expect(screen.getByTestId('network-loading')).toBeInTheDocument();
  });

  it('should render form with fetched network config', async () => {
    await renderNetworkPage();

    // NOSONAR: Hardcoded IP addresses are safe in test files
    expect(screen.getByTestId('network-ip-address-input')).toHaveValue('192.168.1.100'); // NOSONAR
    expect(screen.getByTestId('network-gateway-input')).toHaveValue('192.168.1.1'); // NOSONAR
  });

  it('should toggle DHCP and show/hide static IP fields', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

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
    const user = userEvent.setup();
    await renderNetworkPage();

    await fillFormField(user, 'network-ip-address-input', 'invalid-ip');

    const saveButton = screen.getByTestId('network-save-button');
    await user.click(saveButton);

    // Form validation should prevent submission - dialog should not open
    await waitFor(
      () => {
        expect(screen.queryByTestId('network-confirm-dialog')).not.toBeInTheDocument();
      },
      { timeout: 1000 },
    );
  });

  it('should toggle DNS from DHCP', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    const dnsFromDHCPSwitch = screen.getByTestId('network-dns-from-dhcp-switch');
    expect(dnsFromDHCPSwitch).toBeTruthy();

    await user.click(dnsFromDHCPSwitch);
    await waitFor(() => {
      expect(dnsFromDHCPSwitch).toBeChecked();
    });
  });

  it('should show DNS input fields when DNS from DHCP is off', async () => {
    await renderNetworkPage();

    await waitFor(() => {
      // NOSONAR: Hardcoded IP addresses are safe in test files
      expect(screen.getByTestId('network-primary-dns-input')).toHaveValue('8.8.8.8'); // NOSONAR
      expect(screen.getByTestId('network-secondary-dns-input')).toHaveValue('8.8.4.4'); // NOSONAR
    });
  });

  it('should show confirmation dialog on form submission', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    // NOSONAR: Hardcoded IP address is safe in test file
    await makeFormDirty(user, 'network-ip-address-input', '192.168.1.200'); // NOSONAR

    const saveButton = screen.getByTestId('network-save-button');
    expect(saveButton).toBeTruthy();
    await user.click(saveButton);

    await waitFor(
      () => {
        expect(screen.getByTestId('network-confirm-dialog')).toBeInTheDocument();
        expect(screen.getByTestId('network-confirm-dialog-title')).toBeInTheDocument();
      },
      { timeout: 3000 },
    );
  });

  it('should call mutation on confirmation', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    // NOSONAR: Hardcoded IP address is safe in test file
    await makeFormDirty(user, 'network-ip-address-input', '192.168.1.200'); // NOSONAR

    const saveButton = screen.getByTestId('network-save-button');
    expect(saveButton).toBeTruthy();
    await user.click(saveButton);

    await waitFor(
      () => {
        expect(screen.getByTestId('network-confirm-dialog-title')).toBeInTheDocument();
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
    vi.mocked(setNetworkInterface).mockRejectedValue(new Error('Network error'));

    const user = userEvent.setup();
    renderWithProviders(<NetworkPage />);

    await waitFor(() => {
      expect(screen.getByTestId('network-title')).toBeInTheDocument();
    });

    // Make form dirty
    const ipInput = screen.getByTestId('network-ip-address-input');
    await user.clear(ipInput);
    // NOSONAR: Hardcoded IP address is safe in test file
    await user.type(ipInput, '192.168.1.200'); // NOSONAR

    const saveButton = screen.getByTestId('network-save-button');
    await user.click(saveButton);

    await waitFor(
      () => {
        expect(screen.getByTestId('network-confirm-dialog-title')).toBeInTheDocument();
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
    const user = userEvent.setup();
    await renderNetworkPage();

    // NOSONAR: Hardcoded IP address is safe in test file
    await makeFormDirty(user, 'network-ip-address-input', '192.168.1.200'); // NOSONAR
    const ipInput = screen.getByTestId('network-ip-address-input');

    const resetButton = screen.getByTestId('network-reset-button');
    await user.click(resetButton);

    await waitFor(() => {
      expect(mockToast.info).toHaveBeenCalledWith('Form reset to current values');
      // NOSONAR: Hardcoded IP address is safe in test file
      expect(ipInput).toHaveValue('192.168.1.100'); // NOSONAR
    });
  });

  it('should render connection status card', async () => {
    await renderNetworkPage();

    await waitFor(() => {
      expect(screen.getByTestId('network-mac-address')).toBeInTheDocument();
      // MAC address value is inside the StatusCardItem
      expect(screen.getByTestId('network-mac-address')).toHaveTextContent('00:11:22:33:44:55');
    });
  });
});
