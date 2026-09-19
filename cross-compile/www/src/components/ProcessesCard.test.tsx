/**
 * ProcessesCard Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { type WaitOutcome, waitForCameraBack } from '@/lib/waitForCameraBack';
import { getDiagnostics } from '@/services/diagnosticsService';
import { type ProcessesResponse, getProcesses, restartService } from '@/services/processesService';
import { mockToast, renderWithProviders, waitForPageLoad } from '@/test/componentTestHelpers';

import ProcessesCard from './ProcessesCard';

vi.mock('@/services/processesService', () => ({
  getProcesses: vi.fn(),
  restartService: vi.fn(),
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

    expect(screen.getByTestId('diagnostics-processes-restart-dialog')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-processes-restart-dialog-title')).toHaveTextContent(
      'snmp',
    );
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
    await user.click(screen.getByTestId('diagnostics-processes-restart-confirm'));

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

    expect(
      screen.getByTestId('diagnostics-processes-restart-dialog-description'),
    ).toHaveTextContent('vendor-daemon');
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
    await user.click(screen.getByTestId('diagnostics-processes-restart-confirm'));

    await waitFor(() =>
      expect(screen.getByTestId('diagnostics-processes-reconnecting')).toBeInTheDocument(),
    );
  });
});
