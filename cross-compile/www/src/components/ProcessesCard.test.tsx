/**
 * ProcessesCard Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { type WaitOutcome, waitForCameraBack } from '@/lib/waitForCameraBack';
import { getDiagnostics } from '@/services/diagnosticsService';
import {
  type ProcessesResponse,
  type ServiceStatus,
  getProcesses,
  restartService,
  serviceAction,
} from '@/services/processesService';
import { mockToast, renderWithProviders, waitForPageLoad } from '@/test/componentTestHelpers';

import ProcessesCard from './ProcessesCard';

vi.mock('@/services/processesService', () => ({
  getProcesses: vi.fn(),
  restartService: vi.fn(),
  serviceAction: vi.fn(),
}));

vi.mock('@/services/diagnosticsService', () => ({
  getDiagnostics: vi.fn(),
}));

vi.mock('@/lib/waitForCameraBack', () => ({
  waitForCameraBack: vi.fn(),
  isAbortError: (err: unknown) =>
    err instanceof DOMException
      ? err.name === 'AbortError'
      : err instanceof Error && err.name === 'AbortError',
}));

const data: ProcessesResponse = {
  supervised: [
    { name: 'onvif', state: 'running', pid: 42, uptime_s: 90, restarts: 3, retry_in_s: 0 },
    {
      name: 'vendor-daemon',
      state: 'running',
      pid: 17,
      uptime_s: 3661,
      restarts: 0,
      retry_in_s: 0,
    },
    { name: 'snmp', state: 'backoff', pid: null, uptime_s: 0, restarts: 7, retry_in_s: 12 },
  ],
  processes: [
    { pid: 42, ppid: 1, comm: 'onvif-rust.bin', state: 'S', rss_kb: 8192, cpu_time_s: 3 },
    { pid: 17, ppid: 1, comm: 'vendor-daemon', state: 'R', rss_kb: 4096, cpu_time_s: 1 },
  ],
};

describe('ProcessesCard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getProcesses).mockResolvedValue(data);
    vi.mocked(getDiagnostics).mockResolvedValue({} as Awaited<ReturnType<typeof getDiagnostics>>);
  });

  it('should render pid and uptime for a running service', async () => {
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    await waitFor(() =>
      expect(screen.getByTestId('diagnostics-processes-pid-onvif')).toHaveTextContent('42'),
    );
    expect(screen.getByTestId('diagnostics-processes-uptime-onvif')).toHaveTextContent('1m 30s');
  });

  it('should render the retry countdown and no pid for a backoff service', async () => {
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    await waitFor(() =>
      expect(screen.getByTestId('diagnostics-processes-pid-snmp')).toHaveTextContent('—'),
    );
    expect(screen.getByTestId('diagnostics-processes-retry-countdown-snmp')).toHaveTextContent(
      '12s',
    );
  });

  it('should render the unavailable note and no restart buttons when supervised is null', async () => {
    vi.mocked(getProcesses).mockResolvedValue({
      supervised: null,
      processes: data.processes,
    });

    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    await waitFor(() =>
      expect(screen.getByTestId('diagnostics-processes-supervisor-note')).toBeInTheDocument(),
    );
    expect(screen.queryAllByTestId(/^diagnostics-processes-restart-/)).toHaveLength(0);
  });

  it('should keep the raw process table collapsed by default and expand on click', async () => {
    const user = userEvent.setup();
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    await waitFor(() =>
      expect(screen.getByTestId('diagnostics-processes-raw-trigger')).toBeInTheDocument(),
    );

    expect(screen.queryByTestId('diagnostics-processes-raw-row-42')).not.toBeInTheDocument();
    await user.click(screen.getByTestId('diagnostics-processes-raw-trigger'));
    expect(screen.getByTestId('diagnostics-processes-raw-row-42')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-processes-raw-comm-42')).toHaveTextContent(
      'onvif-rust.bin',
    );
  });

  it('should open a confirm dialog naming the service when restart is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    await waitFor(() =>
      expect(screen.getByTestId('diagnostics-processes-restart-snmp')).toBeInTheDocument(),
    );

    await user.click(screen.getByTestId('diagnostics-processes-restart-snmp'));

    expect(screen.getByTestId('diagnostics-processes-action-dialog')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-processes-action-title')).toHaveTextContent('snmp');
  });

  it('should call restartService with the service name on confirm', async () => {
    const user = userEvent.setup();
    vi.mocked(restartService).mockResolvedValue(undefined);
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    await waitFor(() =>
      expect(screen.getByTestId('diagnostics-processes-restart-snmp')).toBeInTheDocument(),
    );

    await user.click(screen.getByTestId('diagnostics-processes-restart-snmp'));
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));

    await waitFor(() => expect(restartService).toHaveBeenCalledWith('snmp'));
    await waitFor(() =>
      expect(mockToast.success).toHaveBeenCalledWith('Restart requested for snmp'),
    );
  });

  it('should warn that vendor-daemon goes down with onvif', async () => {
    const user = userEvent.setup();
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    await waitFor(() =>
      expect(screen.getByTestId('diagnostics-processes-restart-onvif')).toBeInTheDocument(),
    );

    await user.click(screen.getByTestId('diagnostics-processes-restart-onvif'));

    expect(screen.getByTestId('diagnostics-processes-action-description')).toHaveTextContent(
      'vendor-daemon',
    );
  });

  it('should enter the reconnecting state when onvif is restarted', async () => {
    const user = userEvent.setup();
    // The wait never settles — the point is that the card shows the state.
    const pending = new Promise<WaitOutcome>(() => {});
    vi.mocked(waitForCameraBack).mockReturnValue(
      pending as unknown as ReturnType<typeof waitForCameraBack>,
    );
    vi.mocked(restartService).mockResolvedValue(undefined);
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    await waitFor(() =>
      expect(screen.getByTestId('diagnostics-processes-restart-onvif')).toBeInTheDocument(),
    );

    await user.click(screen.getByTestId('diagnostics-processes-restart-onvif'));
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));

    await waitFor(() =>
      expect(screen.getByTestId('diagnostics-processes-reconnecting')).toBeInTheDocument(),
    );
  });
});

const SNMP: ServiceStatus = {
  name: 'snmp',
  state: 'running',
  pid: 123,
  uptime_s: 500,
  restarts: 0,
  retry_in_s: 0,
};
const OFF: ServiceStatus = {
  name: 'snmp',
  state: 'disabled',
  pid: null,
  uptime_s: 0,
  restarts: 0,
  retry_in_s: 0,
};
const supervisedFixture = (rows: ServiceStatus[]): ProcessesResponse => ({
  supervised: rows,
  processes: [],
});

describe('ProcessesCard service toggling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getDiagnostics).mockResolvedValue({} as Awaited<ReturnType<typeof getDiagnostics>>);
  });

  it('shows Disable for an enabled service and Enable (dimmed) for a disabled one', async () => {
    vi.mocked(getProcesses).mockResolvedValue(
      supervisedFixture([SNMP, { ...OFF, name: 'dropbear' }]),
    );
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    expect(await screen.findByTestId('diagnostics-processes-disable-snmp')).toBeInTheDocument();
    expect(screen.queryByTestId('diagnostics-processes-enable-snmp')).not.toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-processes-enable-dropbear')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-processes-row-dropbear')).toHaveClass('opacity-50');
    expect(screen.getByTestId('diagnostics-processes-status-dropbear')).toHaveTextContent(
      'disabled',
    );
    // A disabled service is not restartable.
    expect(screen.queryByTestId('diagnostics-processes-restart-dropbear')).not.toBeInTheDocument();
  });

  it('offers no toggle for wpa_supplicant', async () => {
    vi.mocked(getProcesses).mockResolvedValue(
      supervisedFixture([{ ...SNMP, name: 'wpa_supplicant' }]),
    );
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    expect(
      await screen.findByTestId('diagnostics-processes-restart-wpa_supplicant'),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId('diagnostics-processes-disable-wpa_supplicant'),
    ).not.toBeInTheDocument();
  });

  it('disabling snmp goes through the confirm dialog', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([SNMP]));
    vi.mocked(serviceAction).mockResolvedValue(undefined);
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-snmp'));
    expect(screen.getByTestId('diagnostics-processes-action-title')).toHaveTextContent(
      'Disable snmp?',
    );
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() => expect(serviceAction).toHaveBeenCalledWith('snmp', 'disable'));
  });

  it('enabling calls serviceAction with enable', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([OFF]));
    vi.mocked(serviceAction).mockResolvedValue(undefined);
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-enable-snmp'));
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() => expect(serviceAction).toHaveBeenCalledWith('snmp', 'enable'));
  });

  it('warns that disabling vendor-daemon stops video', async () => {
    vi.mocked(getProcesses).mockResolvedValue(
      supervisedFixture([{ ...SNMP, name: 'vendor-daemon' }]),
    );
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-vendor-daemon'));
    expect(screen.getByTestId('diagnostics-processes-action-description')).toHaveTextContent(
      'streams go dead',
    );
  });

  it('shows the onvif consequence copy when disabling onvif', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([{ ...SNMP, name: 'onvif' }]));
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-onvif'));
    expect(screen.getByTestId('diagnostics-processes-action-description')).toHaveTextContent(
      'only reachable via FTP',
    );
  });

  it('disabling onvif does NOT enter the reconnecting state, even when the connection drops', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([{ ...SNMP, name: 'onvif' }]));
    vi.mocked(serviceAction).mockRejectedValue(new TypeError('fetch failed'));
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-onvif'));
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() => expect(serviceAction).toHaveBeenCalledWith('onvif', 'disable'));
    // A dropped connection on disable is the expected success path: the card
    // reports it as done, not as an error, and never waits for a comeback.
    expect(screen.queryByTestId('diagnostics-processes-reconnecting')).not.toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByTestId('diagnostics-processes-onvif-off-note')).toBeInTheDocument(),
    );
  });

  it('an ApiError from disable is an error toast with no waiting', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([SNMP]));
    vi.mocked(serviceAction).mockRejectedValue(
      Object.assign(new Error('disable of snmp failed with status 503'), { name: 'ApiError' }),
    );
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-snmp'));
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() =>
      expect(mockToast.error).toHaveBeenCalledWith(expect.stringContaining('503')),
    );
    expect(screen.queryByTestId('diagnostics-processes-reconnecting')).not.toBeInTheDocument();
  });
});

describe('ProcessesCard telnet row', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getDiagnostics).mockResolvedValue({} as Awaited<ReturnType<typeof getDiagnostics>>);
  });

  const TELNET_ON: ServiceStatus = {
    name: 'telnetd',
    state: 'running',
    pid: 99,
    uptime_s: 0,
    restarts: 0,
    retry_in_s: 0,
  };
  const TELNET_OFF: ServiceStatus = {
    name: 'telnetd',
    state: 'disabled',
    pid: null,
    uptime_s: 0,
    restarts: 0,
    retry_in_s: 0,
  };

  it('shows a running telnetd with a Disable action and no Restart', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([SNMP, TELNET_ON]));
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    expect(await screen.findByTestId('diagnostics-processes-status-telnetd')).toHaveTextContent(
      'running',
    );
    expect(screen.getByTestId('diagnostics-processes-disable-telnetd')).toBeInTheDocument();
    // telnetd is not a supervised service: there is no restart to offer.
    expect(screen.queryByTestId('diagnostics-processes-restart-telnetd')).not.toBeInTheDocument();
    // It has no uptime or restart history to show.
    expect(screen.getByTestId('diagnostics-processes-uptime-telnetd')).toHaveTextContent('—');
  });

  it('shows a stopped telnetd dimmed, with Enable and no Restart', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([SNMP, TELNET_OFF]));
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    expect(await screen.findByTestId('diagnostics-processes-enable-telnetd')).toBeInTheDocument();
    expect(screen.queryByTestId('diagnostics-processes-restart-telnetd')).not.toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-processes-row-telnetd')).toHaveClass('opacity-50');
  });

  it('disabling telnetd shows the recovery-channel consequence and calls serviceAction', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([SNMP, TELNET_ON]));
    vi.mocked(serviceAction).mockResolvedValue(undefined);
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-telnetd'));
    expect(screen.getByTestId('diagnostics-processes-action-title')).toHaveTextContent(
      'Disable telnetd?',
    );
    expect(screen.getByTestId('diagnostics-processes-action-description')).toHaveTextContent(
      'recovery telnet (port 24)',
    );
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() => expect(serviceAction).toHaveBeenCalledWith('telnetd', 'disable'));
  });

  it('enabling telnetd warns that it is a root shell on the LAN', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([SNMP, TELNET_OFF]));
    vi.mocked(serviceAction).mockResolvedValue(undefined);
    renderWithProviders(<ProcessesCard />);
    await waitForPageLoad('diagnostics-processes-title');
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-enable-telnetd'));
    expect(screen.getByTestId('diagnostics-processes-action-description')).toHaveTextContent(
      'root shell',
    );
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() => expect(serviceAction).toHaveBeenCalledWith('telnetd', 'enable'));
  });
});
