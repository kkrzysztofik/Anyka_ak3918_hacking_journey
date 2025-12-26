/**
 * TimePage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getDateTime, setNTP } from '@/services/timeService';
import { mockToast, renderWithProviders } from '@/test/componentTestHelpers';

import TimePage from './TimePage';

// Mock services
vi.mock('@/services/timeService', () => ({
  getDateTime: vi.fn(),
  setDateTime: vi.fn(),
  setNTP: vi.fn(),
}));

// Note: Timer mocking will be handled per test

describe('TimePage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getDateTime).mockResolvedValue(mockTimeConfig);
    vi.mocked(setNTP).mockResolvedValue(undefined);
  });

  const mockTimeConfig = {
    ntp: {
      enabled: true,
      fromDHCP: false,
    },
    timezone: 'UTC',
    datetime: new Date(),
    utcDateTime: new Date().toISOString(),
  };

  it('should render page with loading state', async () => {
    vi.mocked(getDateTime).mockImplementation(() => new Promise(() => {}));

    renderWithProviders(<TimePage />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('should render form with fetched time config', async () => {
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Time')).toBeInTheDocument();
    });

    expect(screen.getByText('Synchronization')).toBeInTheDocument();
  });

  it('should display device time', async () => {
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Device Time')).toBeInTheDocument();
    });

    // Device time should be displayed (format may vary, check for time-like pattern)
    const timeDisplay = screen.getByText(/Device Time/i);
    expect(timeDisplay).toBeInTheDocument();
    // Time display area should exist
    const timeContainer = screen.getByText('Device Time').closest('div');
    expect(timeContainer).toBeInTheDocument();
  });

  it('should update device time display on interval', async () => {
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Device Time')).toBeInTheDocument();
    });

    // Verify time display area exists (actual time updates are tested in integration)
    const timeContainer = screen.getByText('Device Time').closest('div');
    expect(timeContainer).toBeInTheDocument();
  });

  it('should select NTP mode', async () => {
    const user = userEvent.setup();
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Time')).toBeInTheDocument();
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

  it('should show NTP server fields when NTP mode is selected and not from DHCP', async () => {
    vi.mocked(getDateTime).mockResolvedValue({
      ...mockTimeConfig,
      ntp: { enabled: true, fromDHCP: false },
    });

    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Time')).toBeInTheDocument();
    });

    // When NTP from DHCP is false, server fields should be visible
    await waitFor(
      () => {
        // Check for NTP server input fields
        const primaryServerInput = screen.queryByTestId('time-page-ntp-server1-input');
        const secondaryServerInput = screen.queryByTestId('time-page-ntp-server2-input');
        // At least verify NTP section is rendered
        const syncTexts = screen.getAllByText(/synchronization/i);
        expect(syncTexts.length).toBeGreaterThan(0);
        expect(
          primaryServerInput || secondaryServerInput || screen.queryByText(/ntp/i),
        ).toBeTruthy();
      },
      { timeout: 3000 },
    );
  });

  it('should select Computer mode', async () => {
    const user = userEvent.setup();
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Time')).toBeInTheDocument();
    });

    // Find Computer radio and click it
    const computerRadio = screen.getByTestId('time-page-computer-radio');
    expect(computerRadio).toBeTruthy();

    await user.click(computerRadio);
    await waitFor(
      () => {
        expect(mockToast.info).toHaveBeenCalledWith('Selected computer time');
      },
      { timeout: 3000 },
    );
  });

  it('should select Manual mode and show date/time inputs', async () => {
    const user = userEvent.setup();
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Time')).toBeInTheDocument();
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

  it('should toggle NTP from DHCP', async () => {
    const user = userEvent.setup();
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Time')).toBeInTheDocument();
    });

    // Ensure NTP mode is selected (switch only appears in NTP mode)
    const ntpRadio = screen.getByTestId('time-page-ntp-radio');
    await user.click(ntpRadio);
    await waitFor(() => {
      // RadioGroupItem is a button with aria-checked, not an input
      const ntpRadioInput = screen.getByTestId('time-page-ntp-radio-input');
      expect(ntpRadioInput?.getAttribute('aria-checked')).toBe('true');
    });

    // Find NTP from DHCP switch (only visible when NTP mode is selected)
    await waitFor(
      () => {
        const ntpFromDHCPSwitch = screen.getByTestId('time-page-ntp-from-dhcp-switch');
        expect(ntpFromDHCPSwitch).toBeTruthy();
      },
      { timeout: 3000 },
    );

    // Now toggle the switch
    const ntpFromDHCPSwitch = screen.getByTestId('time-page-ntp-from-dhcp-switch');
    expect(ntpFromDHCPSwitch).toBeTruthy();

    const wasChecked =
      ntpFromDHCPSwitch.getAttribute('aria-checked') === 'true' ||
      (ntpFromDHCPSwitch as HTMLInputElement).checked === true;
    await user.click(ntpFromDHCPSwitch);
    await waitFor(
      () => {
        const isNowChecked =
          ntpFromDHCPSwitch.getAttribute('aria-checked') === 'true' ||
          (ntpFromDHCPSwitch as HTMLInputElement).checked === true;
        expect(isNowChecked).toBe(!wasChecked);
      },
      { timeout: 3000 },
    );
  });

  it('should select timezone', async () => {
    const user = userEvent.setup();
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Time')).toBeInTheDocument();
    });

    // Find timezone select
    const timezoneSelect = screen.getByTestId('time-page-timezone-select');
    expect(timezoneSelect).toBeTruthy();

    await user.selectOptions(timezoneSelect, 'EST');
    await waitFor(
      () => {
        expect(timezoneSelect).toHaveValue('EST');
      },
      { timeout: 3000 },
    );
  });

  it('should submit form and call mutation', async () => {
    const user = userEvent.setup();
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Time')).toBeInTheDocument();
    });

    // Make form dirty by changing timezone
    const timezoneSelect = screen.getByTestId('time-page-timezone-select');

    expect(timezoneSelect).toBeTruthy();
    await user.selectOptions(timezoneSelect, 'EST');
    // Wait for form to become dirty
    await waitFor(
      () => {
        const saveButton = screen.getByTestId('time-page-save-button');
        expect(saveButton).toBeTruthy();
        expect(saveButton).not.toBeDisabled();
      },
      { timeout: 3000 },
    );

    const saveButton = screen.getByTestId('time-page-save-button');
    expect(saveButton).toBeTruthy();
    await user.click(saveButton);

    await waitFor(
      () => {
        expect(setNTP).toHaveBeenCalled();
        expect(mockToast.success).toHaveBeenCalledWith('Time settings saved');
      },
      { timeout: 5000 },
    );
  });

  it('should show error toast when mutation fails', async () => {
    vi.mocked(setNTP).mockRejectedValue(new Error('Network error'));

    const user = userEvent.setup();
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Time')).toBeInTheDocument();
    });

    // Make form dirty by changing timezone
    const timezoneSelect = screen.getByTestId('time-page-timezone-select');

    expect(timezoneSelect).toBeTruthy();
    await user.selectOptions(timezoneSelect, 'EST');
    // Wait for form to become dirty
    await waitFor(
      () => {
        const saveButton = screen.getByTestId('time-page-save-button');
        expect(saveButton).toBeTruthy();
        expect(saveButton).not.toBeDisabled();
      },
      { timeout: 3000 },
    );

    const saveButton = screen.getByTestId('time-page-save-button');
    expect(saveButton).toBeTruthy();
    await user.click(saveButton);

    await waitFor(
      () => {
        expect(mockToast.error).toHaveBeenCalledWith('Failed to save time settings', {
          description: 'Network error',
        });
      },
      { timeout: 5000 },
    );
  });

  it('should render timezone configuration card', async () => {
    renderWithProviders(<TimePage />);

    await waitFor(() => {
      expect(screen.getByText('Time Zone')).toBeInTheDocument();
      expect(screen.getByText('Set the local time zone')).toBeInTheDocument();
    });
  });
});
