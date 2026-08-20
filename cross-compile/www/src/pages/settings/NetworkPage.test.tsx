/**
 * NetworkPage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getDiagnostics } from '@/services/diagnosticsService';
import {
  getNetworkConfig,
  getNetworkInterfaces,
  getNetworkOverlay,
  putNetworkOverlay,
  setDNS,
  setNetworkDefaultGateway,
  setNetworkInterface,
  setNetworkProtocols,
} from '@/services/networkService';
import {
  MOCK_DATA,
  fillFormField,
  makeFormDirty,
  mockToast,
  renderWithProviders,
  waitForPageLoad,
} from '@/test/componentTestHelpers';

import NetworkPage from './NetworkPage';

vi.mock('@/services/networkService', () => ({
  getNetworkConfig: vi.fn(),
  getNetworkInterfaces: vi.fn(),
  getNetworkOverlay: vi.fn(),
  putNetworkOverlay: vi.fn(),
  setNetworkInterface: vi.fn(),
  setNetworkDefaultGateway: vi.fn(),
  setDNS: vi.fn(),
  setNetworkProtocols: vi.fn(),
}));

vi.mock('@/services/diagnosticsService', () => ({
  getDiagnostics: vi.fn(),
}));

const MOCK_DIAGNOSTICS = {
  status: 'healthy',
  firmware_version: 'test',
  uptime: { process_s: 100, system_s: 3600 },
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
    signal_dbm: -44,
    link_quality: '66/70',
  },
};

const EMPTY_OVERLAY = {
  pending: { has_password: false },
  has_pending: false,
  last_failure: null,
};

describe('NetworkPage', () => {
  const renderNetworkPage = async () => {
    const result = renderWithProviders(<NetworkPage />);
    await waitForPageLoad('network-title');
    return result;
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getNetworkConfig).mockResolvedValue(MOCK_DATA.network);
    vi.mocked(getNetworkOverlay).mockResolvedValue(EMPTY_OVERLAY);
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
    vi.mocked(getDiagnostics).mockResolvedValue(MOCK_DIAGNOSTICS);
    vi.mocked(putNetworkOverlay).mockResolvedValue(undefined);
    vi.mocked(setNetworkInterface).mockResolvedValue(undefined);
    vi.mocked(setNetworkDefaultGateway).mockResolvedValue(undefined);
    vi.mocked(setDNS).mockResolvedValue(undefined);
    vi.mocked(setNetworkProtocols).mockResolvedValue(undefined);
  });

  it('should render page with loading state', async () => {
    vi.mocked(getNetworkConfig).mockImplementation(() => new Promise(() => {}));

    renderWithProviders(<NetworkPage />);
    expect(screen.getByTestId('network-loading')).toBeInTheDocument();
  });

  it('should render form with fetched network config', async () => {
    await renderNetworkPage();

    expect(screen.getByTestId('network-ip-address-input')).toHaveValue('192.168.1.100');
    expect(screen.getByTestId('network-gateway-input')).toHaveValue('192.168.1.1');
  });

  it('should not render a hostname input; Identification owns it', async () => {
    await renderNetworkPage();
    expect(screen.queryByTestId('network-hostname-input')).toBeNull();
  });

  it('should not render an ONVIF discovery switch; Identification owns it', async () => {
    await renderNetworkPage();
    expect(screen.queryByTestId('network-onvif-discovery-switch')).toBeNull();
  });

  it('should link to the Identification pane instead', async () => {
    await renderNetworkPage();
    expect(screen.getByTestId('network-identification-link')).toHaveAttribute(
      'href',
      '#/settings/identification',
    );
  });

  it('should not render an HTTPS port input; no TLS listener exists', async () => {
    await renderNetworkPage();
    expect(screen.queryByTestId('network-https-port-input')).toBeNull();
  });

  it('should badge IP Configuration as pending when the overlay differs from live', async () => {
    vi.mocked(getNetworkOverlay).mockResolvedValue({
      pending: { has_password: false, dhcp: false, address: '192.168.2.50/24' },
      has_pending: true,
      last_failure: null,
    });

    await renderNetworkPage();
    expect(await screen.findByTestId('network-ip-pending-badge')).toBeInTheDocument();
  });

  it('should not badge IP Configuration when only Wi-Fi credentials are pending', async () => {
    vi.mocked(getNetworkOverlay).mockResolvedValue({
      pending: { has_password: false, ssid: 'OtherNet' },
      has_pending: true,
      last_failure: null,
    });

    await renderNetworkPage();
    await waitFor(() => expect(screen.queryByTestId('network-ip-pending-badge')).toBeNull());
  });

  it('should show no badge when the overlay matches the live config', async () => {
    await renderNetworkPage();
    await waitFor(() => expect(screen.queryByTestId('network-ip-pending-badge')).toBeNull());
  });

  it('should warn when the previous Wi-Fi settings failed and were reverted', async () => {
    vi.mocked(getNetworkOverlay).mockResolvedValue({
      pending: { has_password: false },
      has_pending: false,
      last_failure: { has_password: false, ssid: 'TypoNet' },
    });

    await renderNetworkPage();
    expect(await screen.findByTestId('network-failure-banner')).toHaveTextContent('TypoNet');
  });

  it('should show confirmation dialog on form submission', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    await makeFormDirty(user, 'network-ip-address-input', '192.168.1.200');
    await user.click(screen.getByTestId('network-save-button'));

    expect(await screen.findByTestId('network-confirm-dialog')).toBeInTheDocument();
  });

  it('should spell out the new URL when the HTTP port changes', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    await makeFormDirty(user, 'network-http-port-input', '8080');
    await user.click(screen.getByTestId('network-save-button'));

    expect(await screen.findByTestId('network-confirm-dialog')).toHaveTextContent(':8080');
  });

  it('should call mutation on confirmation', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    await fillFormField(user, 'network-ssid-input', 'MyNet');
    await makeFormDirty(user, 'network-ip-address-input', '192.168.1.200');
    await user.click(screen.getByTestId('network-save-button'));
    await user.click(await screen.findByTestId('network-confirm-save-button'));

    await waitFor(() => {
      expect(putNetworkOverlay).toHaveBeenCalled();
      expect(setNetworkInterface).toHaveBeenCalled();
      expect(setDNS).toHaveBeenCalled();
      expect(mockToast.success).toHaveBeenCalledWith(
        'Network settings saved',
        expect.objectContaining({ description: expect.stringContaining('reboot') }),
      );
    });
  });

  it('should skip the Wi-Fi overlay patch when only IP settings change', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    await makeFormDirty(user, 'network-ip-address-input', '192.168.1.200');
    await user.click(screen.getByTestId('network-save-button'));
    await user.click(await screen.findByTestId('network-confirm-save-button'));

    await waitFor(() => {
      expect(setNetworkInterface).toHaveBeenCalled();
      expect(putNetworkOverlay).not.toHaveBeenCalled();
    });
  });

  it('should allow an IP-only save when no SSID is available', async () => {
    vi.mocked(getDiagnostics).mockResolvedValue({
      ...MOCK_DIAGNOSTICS,
      wifi: { ...MOCK_DIAGNOSTICS.wifi!, ssid: '' },
    });

    const user = userEvent.setup();
    await renderNetworkPage();

    await makeFormDirty(user, 'network-ip-address-input', '192.168.1.200');
    await user.click(screen.getByTestId('network-save-button'));
    await user.click(await screen.findByTestId('network-confirm-save-button'));

    await waitFor(() => {
      expect(setNetworkInterface).toHaveBeenCalled();
      expect(putNetworkOverlay).not.toHaveBeenCalled();
    });
  });

  it('should reject static IP save when address is empty with DHCP disabled', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    expect(await screen.findByTestId('network-ip-address-input')).toBeInTheDocument();
    await user.clear(screen.getByTestId('network-ip-address-input'));
    await user.clear(screen.getByTestId('network-gateway-input'));
    await user.click(screen.getByTestId('network-save-button'));

    await waitFor(() => {
      expect(screen.queryByTestId('network-confirm-dialog')).toBeNull();
    });
    expect(putNetworkOverlay).not.toHaveBeenCalled();
  });

  it('should report failure when a save faults', async () => {
    vi.mocked(setDNS).mockRejectedValue(new Error('ActionNotSupported'));

    const user = userEvent.setup();
    await renderNetworkPage();

    await fillFormField(user, 'network-ssid-input', 'MyNet');
    await makeFormDirty(user, 'network-ip-address-input', '192.168.1.200');
    await user.click(screen.getByTestId('network-save-button'));
    await user.click(await screen.findByTestId('network-confirm-save-button'));

    await waitFor(() => expect(mockToast.error).toHaveBeenCalled());
    expect(mockToast.success).not.toHaveBeenCalled();
  });

  it('should report which part failed when the interface saves but DNS does not', async () => {
    vi.mocked(setNetworkInterface).mockResolvedValue(undefined);
    vi.mocked(setDNS).mockRejectedValue(new Error('boom'));

    const user = userEvent.setup();
    await renderNetworkPage();

    await fillFormField(user, 'network-ssid-input', 'MyNet');
    await makeFormDirty(user, 'network-ip-address-input', '192.168.1.200');
    await user.click(screen.getByTestId('network-save-button'));
    await user.click(await screen.findByTestId('network-confirm-save-button'));

    await waitFor(() =>
      expect(mockToast.error).toHaveBeenCalledWith(
        'Failed to save settings',
        expect.objectContaining({ description: expect.stringContaining('DNS') }),
      ),
    );
  });

  it('should reset form when reset button is clicked', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    await makeFormDirty(user, 'network-ip-address-input', '192.168.1.200');
    const ipInput = screen.getByTestId('network-ip-address-input');
    await user.click(screen.getByTestId('network-reset-button'));

    await waitFor(() => {
      expect(mockToast.info).toHaveBeenCalledWith('Form reset to current values');
      expect(ipInput).toHaveValue('192.168.1.100');
    });
  });
});
