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
  getSnmpConfig,
  putNetworkOverlay,
  putSnmpConfig,
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
  getSnmpConfig: vi.fn(),
  putNetworkOverlay: vi.fn(),
  putSnmpConfig: vi.fn(),
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
    vi.mocked(getSnmpConfig).mockResolvedValue({
      enabled: true,
      port: 161,
      community: 'public',
      sys_contact: '',
      sys_name: '',
      sys_location: '',
    });
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
    vi.mocked(putSnmpConfig).mockResolvedValue(undefined);
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

  it.each([
    ['network-hostname-input', 'hostname; Identification owns it'],
    ['network-onvif-discovery-switch', 'ONVIF discovery; Identification owns it'],
    ['network-https-port-input', 'HTTPS port; no TLS listener exists'],
  ] as const)('should not render %s (%s)', async (testId, _reason) => {
    await renderNetworkPage();
    expect(screen.queryByTestId(testId)).toBeNull();
  });

  it('should link to the Identification pane instead', async () => {
    await renderNetworkPage();
    expect(screen.getByTestId('network-identification-link')).toHaveAttribute(
      'href',
      '#/settings/identification',
    );
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
    const live = MOCK_DATA.network;
    const iface = live.interfaces[0];
    vi.mocked(getNetworkOverlay).mockResolvedValue({
      pending: {
        has_password: false,
        dhcp: iface.dhcp,
        address: `${iface.address}/${iface.prefixLength}`,
        gateway: iface.gateway,
        dns: [...live.dns.dnsServers],
      },
      has_pending: true,
      last_failure: null,
    });

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

  it('test_render_snmp_settings_fetched_config_displays_values', async () => {
    vi.mocked(getSnmpConfig).mockResolvedValue({
      enabled: true,
      port: 1161,
      community: 'monitor',
      sys_contact: '',
      sys_name: '',
      sys_location: '',
    });

    await renderNetworkPage();

    expect(screen.getByTestId('network-snmp-port-input')).toHaveValue(1161);
    expect(screen.getByTestId('network-snmp-community-input')).toHaveValue('monitor');
  });

  it('test_save_snmp_settings_on_confirmation_calls_putSnmpConfig', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    await makeFormDirty(user, 'network-snmp-port-input', '2161');
    await user.click(screen.getByTestId('network-save-button'));
    await user.click(await screen.findByTestId('network-confirm-save-button'));

    await waitFor(() => {
      expect(putSnmpConfig).toHaveBeenCalledWith(
        expect.objectContaining({ enabled: true, port: 2161, community: 'public' }),
      );
    });
  });

  it('test_save_empty_community_blocks_submit_and_skips_putSnmpConfig', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    await user.clear(screen.getByTestId('network-snmp-community-input'));
    await user.click(screen.getByTestId('network-save-button'));

    expect(await screen.findByTestId('network-snmp-community-error')).toHaveTextContent(
      'Community must not be empty',
    );
    expect(putSnmpConfig).not.toHaveBeenCalled();
  });

  it('test_confirm_dialog_snmp_only_save_shows_reload_message', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    await makeFormDirty(user, 'network-snmp-port-input', '2161');
    await user.click(screen.getByTestId('network-save-button'));

    expect(await screen.findByTestId('network-confirm-dialog-description')).toHaveTextContent(
      'SNMP changes apply on reload without reboot.',
    );
  });

  it('test_snmp_load_error_disables_snmp_fields', async () => {
    vi.mocked(getSnmpConfig).mockRejectedValue(new Error('SNMP unavailable'));
    await renderNetworkPage();

    expect(await screen.findByTestId('network-snmp-load-error')).toHaveTextContent(
      'SNMP unavailable',
    );
    expect(screen.getByTestId('network-snmp-port-input')).toBeDisabled();
    expect(screen.getByTestId('network-snmp-community-input')).toBeDisabled();
  });

  it('test_save_skips_snmp_put_when_config_unavailable', async () => {
    vi.mocked(getSnmpConfig).mockRejectedValue(new Error('SNMP unavailable'));
    const user = userEvent.setup();
    await renderNetworkPage();
    await screen.findByTestId('network-snmp-load-error');

    await makeFormDirty(user, 'network-http-port-input', '8080');
    await user.click(screen.getByTestId('network-save-button'));
    await user.click(await screen.findByTestId('network-confirm-save-button'));

    await waitFor(() => {
      expect(setNetworkProtocols).toHaveBeenCalled();
    });
    expect(putSnmpConfig).not.toHaveBeenCalled();
  });
});
