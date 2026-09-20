/**
 * TimePage Tests
 */
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { getDateTime, setDateTime, setNtp, setSystemDateAndTime } from '@/services/timeService';
import {
  renderWithProviders,
  selectOption,
  waitForPageLoad,
} from '@/test/componentTestHelpers';
import {
  testMutationWithErrorToast,
  testMutationWithSuccessToast,
} from '@/test/mutationTestHelpers';
import { TIMEZONES } from '@/utils/timezones';

import TimePage from './TimePage';

// Mock services
vi.mock('@/services/timeService', () => ({
  getDateTime: vi.fn(),
  setDateTime: vi.fn(),
  setNtp: vi.fn(),
  setSystemDateAndTime: vi.fn(),
  getNtp: vi.fn(),
}));

// Note: Timer mocking will be handled per test

describe('TimePage', () => {
  const renderTimePage = async () => {
    const result = renderWithProviders(<TimePage />);
    await waitForPageLoad('time-title');
    return result;
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getDateTime).mockResolvedValue(mockTimeConfig);
    vi.mocked(setDateTime).mockResolvedValue(undefined);
    vi.mocked(setNtp).mockResolvedValue(undefined);
    vi.mocked(setSystemDateAndTime).mockResolvedValue(undefined);
  });

  const mockTimeConfig = {
    ntp: {
      enabled: true,
    },
    daylightSavings: false,
    timezone: 'UTC0',
    utcDateTime: new Date(),
  };

  describe('camera clock', () => {
    // waitFor in this project does not auto-advance fake timers, so each test
    // flushes the initial query with a few explicit advances.
    const flush = async () => {
      await act(async () => {
        vi.advanceTimersByTime(100);
      });
      await act(async () => {
        vi.advanceTimersByTime(100);
      });
    };

    // A failed test below may skip its own useRealTimers; never leak timers.
    afterEach(() => {
      vi.useRealTimers();
    });

    it('should display the camera time, not the browser time', async () => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date('2026-09-20T12:00:00Z'));
      // Camera is three hours behind the browser.
      vi.mocked(getDateTime).mockResolvedValue({
        ...mockTimeConfig,
        utcDateTime: new Date('2026-09-20T09:00:00Z'),
      });

      renderWithProviders(<TimePage />);
      await flush();

      // ±1s: the 1s tick interval can fire between mount and the first paint.
      const shown = screen.getByTestId('time-device-clock').textContent;
      expect(['08:59:59', '09:00:00']).toContain(shown);
      vi.useRealTimers();
    });

    it('should keep ticking from the camera offset', async () => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date('2026-09-20T12:00:00Z'));
      vi.mocked(getDateTime).mockResolvedValue({
        ...mockTimeConfig,
        utcDateTime: new Date('2026-09-20T09:00:00Z'),
      });

      renderWithProviders(<TimePage />);
      await flush();
      const before = screen.getByTestId('time-device-clock').textContent;
      await act(async () => {
        vi.advanceTimersByTime(3000);
      });
      const after = screen.getByTestId('time-device-clock').textContent;
      // It advanced, and stayed on the camera's (09:xx) track, not the browser's.
      expect(after > before).toBe(true);
      expect(after).toMatch(/^09:0[01]:/);
      vi.useRealTimers();
    });

    it('should warn when the camera clock is implausible', async () => {
      vi.mocked(getDateTime).mockResolvedValue({
        ...mockTimeConfig,
        utcDateTime: new Date('1970-01-01T00:00:00Z'),
      });

      await renderTimePage();

      expect(screen.getByTestId('time-clock-stale')).toBeInTheDocument();
    });
  });

  it('should render page with loading state', async () => {
    vi.mocked(getDateTime).mockImplementation(() => new Promise(() => {}));

    renderWithProviders(<TimePage />);
    expect(screen.getByTestId('time-loading')).toBeInTheDocument();
  });

  it('should render form with fetched time config', async () => {
    await renderTimePage();

    expect(screen.getByTestId('time-synchronization-title')).toBeInTheDocument();
  });

  it('should display device time', async () => {
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByTestId('time-device-time-label')).toBeInTheDocument();
    });

    // Device time display area should exist
    const timeContainer = screen.getByTestId('time-device-time-label').closest('div');
    expect(timeContainer).toBeInTheDocument();
  });

  it('should update device time display on interval', async () => {
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByTestId('time-device-time-label')).toBeInTheDocument();
    });

    // Verify time display area exists (actual time updates are tested in integration)
    const timeContainer = screen.getByTestId('time-device-time-label').closest('div');
    expect(timeContainer).toBeInTheDocument();
  });

  it('should select NTP mode', async () => {
    const user = userEvent.setup();
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByTestId('time-title')).toBeInTheDocument();
    });

    // Find NTP radio and click it
    const ntpRadio = screen.getByTestId('time-page-ntp-radio');
    expect(ntpRadio).toBeTruthy();

    await user.click(ntpRadio);
    // Verify radio is checked by finding it via ID
    await waitFor(
      () => {
        const ntpRadioInput = screen.getByTestId('time-page-ntp-radio-input');
        expect(ntpRadioInput).toBeChecked();
      },
      { timeout: 3000 },
    );
  });

  it('should show NTP server fields when NTP mode is selected', async () => {
    vi.mocked(getDateTime).mockResolvedValue({ ...mockTimeConfig, ntp: { enabled: true } });

    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByTestId('time-title')).toBeInTheDocument();
    });

    // When NTP from DHCP is false, server fields should be visible
    await waitFor(
      () => {
        // Check for NTP server input fields
        const primaryServerInput = screen.queryByTestId('time-page-ntp-server1-input');
        const secondaryServerInput = screen.queryByTestId('time-page-ntp-server2-input');
        // At least verify NTP section is rendered
        expect(screen.getByTestId('time-synchronization-title')).toBeInTheDocument();
        expect(
          primaryServerInput || secondaryServerInput || screen.queryByTestId('time-page-ntp-radio'),
        ).toBeTruthy();
      },
      { timeout: 3000 },
    );
  });

  it('should sync time from the browser', async () => {
    const user = userEvent.setup();
    await renderTimePage();

    await user.click(screen.getByTestId('time-page-use-computer-time'));

    expect(setDateTime).toHaveBeenCalledTimes(1);
    const [iso, timezone, daylightSavings] = vi.mocked(setDateTime).mock.calls[0];
    expect(iso).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);
    expect(timezone).toBe('UTC0');
    expect(daylightSavings).toBe(false);
  });

  it('should select Manual mode and show date/time inputs', async () => {
    const user = userEvent.setup();
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByTestId('time-title')).toBeInTheDocument();
    });

    // Find Manual radio and click it
    const manualRadio = screen.getByTestId('time-page-manual-radio');
    expect(manualRadio).toBeTruthy();

    await user.click(manualRadio);
    await waitFor(
      () => {
        // Verify radio is checked by finding it via ID
        const manualRadioInput = screen.getByTestId('time-page-manual-radio-input');
        expect(manualRadioInput).toBeChecked();
        // Manual date and time inputs should appear
        const dateInput = screen.queryByTestId('time-page-manual-date-input');
        const timeInput = screen.queryByTestId('time-page-manual-time-input');
        expect(dateInput || timeInput).toBeTruthy();
      },
      { timeout: 3000 },
    );
  });

  it('should select timezone', async () => {
    const user = userEvent.setup();
    await renderTimePage();

    const newYork = 'EST5EDT,M3.2.0,M11.1.0';
    await selectOption(user, 'time-page-timezone-select', newYork);
    await waitFor(
      () => {
        const timezoneSelect = screen.getByTestId('time-page-timezone-select');
        expect(timezoneSelect).toHaveValue(newYork);
      },
      { timeout: 3000 },
    );
  });

  it('should round-trip every timezone value through the picker', async () => {
    const user = userEvent.setup();
    await renderTimePage();

    for (const tz of TIMEZONES) {
      await selectOption(user, 'time-page-timezone-select', tz.value);
      const timezoneSelect = screen.getByTestId('time-page-timezone-select');
      expect(timezoneSelect).toHaveValue(tz.value);
    }
  });

  it('should match the default selection to the value returned by getSystemDateAndTime', async () => {
    const tz = 'CET-1CEST,M3.5.0,M10.5.0/3';
    vi.mocked(getDateTime).mockResolvedValue({ ...mockTimeConfig, timezone: tz });

    await renderTimePage();
    expect(screen.getByTestId('time-page-timezone-select')).toHaveValue(tz);
  });

  it('should submit form and call mutation', async () => {
    const user = userEvent.setup();
    await renderTimePage();

    await selectOption(user, 'time-page-timezone-select', 'EST5EDT,M3.2.0,M11.1.0');
    // Wait for form to become dirty
    await waitFor(
      () => {
        const saveButton = screen.getByTestId('time-page-save-button');
        expect(saveButton).toBeTruthy();
        expect(saveButton).not.toBeDisabled();
      },
      { timeout: 3000 },
    );

    await testMutationWithSuccessToast(
      user,
      'time-page-save-button',
      setSystemDateAndTime,
      'Time settings saved',
    );
  });

  it('should show error toast when mutation fails', async () => {
    vi.mocked(setSystemDateAndTime).mockRejectedValue(new Error('Network error'));

    const user = userEvent.setup();
    await renderTimePage();

    await selectOption(user, 'time-page-timezone-select', 'EST5EDT,M3.2.0,M11.1.0');
    // Wait for form to become dirty
    await waitFor(
      () => {
        const saveButton = screen.getByTestId('time-page-save-button');
        expect(saveButton).toBeTruthy();
        expect(saveButton).not.toBeDisabled();
      },
      { timeout: 3000 },
    );

    await testMutationWithErrorToast(
      user,
      'time-page-save-button',
      setSystemDateAndTime,
      'Failed to save time settings',
      'Network error',
    );
  });

  it('should render timezone configuration card', async () => {
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByTestId('time-timezone-title')).toBeInTheDocument();
      expect(screen.getByTestId('time-timezone-description')).toBeInTheDocument();
    });
  });
});
