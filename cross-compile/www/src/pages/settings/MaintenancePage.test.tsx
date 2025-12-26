/**
 * MaintenancePage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  getSystemBackup,
  restoreSystem,
  setSystemFactoryDefault,
  systemReboot,
} from '@/services/maintenanceService';
import { mockToast, renderWithProviders } from '@/test/componentTestHelpers';

import MaintenancePage from './MaintenancePage';

// Mock services
vi.mock('@/services/maintenanceService', () => ({
  systemReboot: vi.fn(),
  setSystemFactoryDefault: vi.fn(),
  getSystemBackup: vi.fn(),
  restoreSystem: vi.fn(),
}));

// Mock URL.createObjectURL and document methods
globalThis.URL.createObjectURL = vi.fn(() => 'blob:mock-url');
globalThis.URL.revokeObjectURL = vi.fn();

// Store original createElement
const originalCreateElement = document.createElement.bind(document);

// Mock createElement for specific cases
globalThis.document.createElement = vi.fn((tag: string) => {
  if (tag === 'a') {
    const element = originalCreateElement('a');
    element.click = vi.fn();
    element.remove = vi.fn();
    return element;
  }
  if (tag === 'input') {
    const element = originalCreateElement('input');
    element.click = vi.fn();
    return element;
  }
  return originalCreateElement(tag);
}) as typeof document.createElement;

describe('MaintenancePage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(systemReboot).mockResolvedValue(undefined);
    vi.mocked(setSystemFactoryDefault).mockResolvedValue(undefined);
    vi.mocked(restoreSystem).mockResolvedValue(undefined);
    vi.mocked(getSystemBackup).mockResolvedValue([]);
  });

  it('should render all maintenance operation cards', () => {
    renderWithProviders(<MaintenancePage />);
    expect(screen.getByText('Maintenance')).toBeInTheDocument();
    expect(screen.getByText('Configuration Backup & Restore')).toBeInTheDocument();
    expect(screen.getByText('Firmware')).toBeInTheDocument();
    expect(screen.getByText('Soft factory reset')).toBeInTheDocument();
    expect(screen.getByText('Hard factory reset')).toBeInTheDocument();
    expect(screen.getByText('Reboot Device')).toBeInTheDocument();
  });

  it('should open and close reboot dialog', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const rebootButton = screen.getByTestId('maintenance-reboot-button');
    await user.click(rebootButton);

    await waitFor(() => {
      expect(screen.getByText('Reboot Device?')).toBeInTheDocument();
    });

    const cancelButton = screen.getByTestId('maintenance-reboot-cancel-button');
    await user.click(cancelButton);

    await waitFor(() => {
      expect(screen.queryByText('Reboot Device?')).not.toBeInTheDocument();
    });
  });

  it('should call reboot mutation on confirm', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const rebootButton = screen.getByTestId('maintenance-reboot-button');
    await user.click(rebootButton);

    await waitFor(() => {
      expect(screen.getByText('Reboot Device?')).toBeInTheDocument();
    });

    const confirmButton = screen.getByTestId('maintenance-reboot-confirm-button');
    await user.click(confirmButton);

    await waitFor(() => {
      expect(systemReboot).toHaveBeenCalled();
      expect(mockToast.success).toHaveBeenCalledWith('Device is rebooting', {
        description: 'Please wait for the device to restart...',
        duration: 10000,
      });
    });
  });

  it('should show error toast when reboot fails', async () => {
    vi.mocked(systemReboot).mockRejectedValue(new Error('Network error'));

    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const rebootButton = screen.getByTestId('maintenance-reboot-button');
    await user.click(rebootButton);

    await waitFor(() => {
      expect(screen.getByText('Reboot Device?')).toBeInTheDocument();
    });

    const confirmButton = screen.getByTestId('maintenance-reboot-confirm-button');
    await user.click(confirmButton);

    await waitFor(() => {
      expect(mockToast.error).toHaveBeenCalledWith('Failed to reboot device', {
        description: 'Network error',
      });
    });
  });

  it('should open and close soft reset dialog', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const softResetButton = screen.getByTestId('maintenance-soft-reset-button');
    await user.click(softResetButton);

    await waitFor(() => {
      expect(screen.getByText('Soft Reset')).toBeInTheDocument();
    });

    const cancelButton = screen.getByTestId('maintenance-soft-reset-cancel-button');
    await user.click(cancelButton);

    await waitFor(() => {
      expect(screen.queryByText('Soft Reset')).not.toBeInTheDocument();
    });
  });

  it('should call soft reset mutation on confirm', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const softResetButton = screen.getByTestId('maintenance-soft-reset-button');
    await user.click(softResetButton);

    await waitFor(() => {
      expect(screen.getByText('Soft Reset')).toBeInTheDocument();
    });

    const confirmButton = screen.getByTestId('maintenance-soft-reset-confirm-button');
    await user.click(confirmButton);

    await waitFor(() => {
      expect(setSystemFactoryDefault).toHaveBeenCalledWith('Soft');
      expect(mockToast.success).toHaveBeenCalledWith('Device is resetting', {
        description: 'Settings returned to defaults. Device will reboot.',
        duration: 10000,
      });
    });
  });

  it('should open and close hard reset dialog', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const hardResetButton = screen.getByTestId('maintenance-hard-reset-button');
    await user.click(hardResetButton);

    await waitFor(() => {
      // Dialog title should appear (not the button)
      const factoryResetTexts = screen.getAllByText('Factory Reset');
      // Should have at least the dialog title
      expect(factoryResetTexts.length).toBeGreaterThan(0);
    });

    const cancelButton = screen.getByTestId('maintenance-hard-reset-cancel-button');
    await user.click(cancelButton);

    await waitFor(() => {
      // Dialog should close - check that dialog content is gone
      const factoryResetTexts = screen.queryAllByText('Factory Reset');
      // Only the button should remain
      expect(factoryResetTexts.length).toBeLessThanOrEqual(1);
    });
  });

  it('should call hard reset mutation on confirm', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const hardResetButton = screen.getByTestId('maintenance-hard-reset-button');
    await user.click(hardResetButton);

    await waitFor(() => {
      // Dialog should be open - check for dialog title
      const factoryResetTexts = screen.getAllByText('Factory Reset');
      expect(factoryResetTexts.length).toBeGreaterThan(0);
    });

    const confirmButton = screen.getByTestId('maintenance-hard-reset-confirm-button');
    await user.click(confirmButton);

    await waitFor(() => {
      expect(setSystemFactoryDefault).toHaveBeenCalledWith('Hard');
      expect(mockToast.success).toHaveBeenCalledWith('Factory reset initiated', {
        description: 'All data will be erased. Device will reboot.',
        duration: 15000,
      });
    });
  });

  it('should trigger backup download', async () => {
    const mockBackupFiles = [
      {
        Name: 'config.toml',
        Data: btoa('test config content'),
      },
    ];
    vi.mocked(getSystemBackup).mockResolvedValue(mockBackupFiles);

    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const backupButton = screen.getByTestId('maintenance-backup-button');
    await user.click(backupButton);

    await waitFor(() => {
      expect(getSystemBackup).toHaveBeenCalled();
      expect(mockToast.success).toHaveBeenCalled();
    });
  });

  it('should show error when backup fails', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const backupButton = screen.getByTestId('maintenance-backup-button');
    await user.click(backupButton);

    await waitFor(() => {
      expect(mockToast.error).toHaveBeenCalledWith('Backup failed', {
        description: 'No backup files received from device',
      });
    });
  });

  it('should trigger restore upload', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const restoreButton = screen.getByTestId('maintenance-restore-button');
    await user.click(restoreButton);

    // File input is created and clicked programmatically
    await waitFor(() => {
      expect(document.createElement).toHaveBeenCalledWith('input');
    });
  });

  it('should show upgrade info when upgrade button is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const upgradeButton = screen.getByTestId('maintenance-upgrade-button');
    await user.click(upgradeButton);

    await waitFor(() => {
      expect(mockToast.info).toHaveBeenCalledWith('Firmware upgrade not available');
    });
  });

  it('should show error when soft reset fails', async () => {
    vi.mocked(setSystemFactoryDefault).mockRejectedValue(new Error('Reset failed'));

    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const softResetButton = screen.getByTestId('maintenance-soft-reset-button');
    await user.click(softResetButton);

    await waitFor(() => {
      expect(screen.getByText('Soft Reset')).toBeInTheDocument();
    });

    const confirmButton = screen.getByTestId('maintenance-soft-reset-confirm-button');
    await user.click(confirmButton);

    await waitFor(() => {
      expect(mockToast.error).toHaveBeenCalledWith('Failed to reset settings', {
        description: 'Reset failed',
      });
    });
  });

  it('should show error when hard reset fails', async () => {
    vi.mocked(setSystemFactoryDefault).mockRejectedValue(new Error('Factory reset failed'));

    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const hardResetButtons = screen.getAllByRole('button');
    const hardResetButton = hardResetButtons.find((btn) =>
      btn.textContent?.toLowerCase().includes('factory reset'),
    );

    if (hardResetButton) {
      await user.click(hardResetButton);
    }

    await waitFor(() => {
      const factoryResetTexts = screen.getAllByText('Factory Reset');
      expect(factoryResetTexts.length).toBeGreaterThan(0);
    });

    const confirmButtons = screen.getAllByRole('button');
    const confirmButton = confirmButtons.find((btn) =>
      btn.textContent?.toLowerCase().includes('erase everything'),
    );

    if (confirmButton) {
      await user.click(confirmButton);
    }

    await waitFor(() => {
      expect(mockToast.error).toHaveBeenCalledWith('Failed to factory reset', {
        description: 'Factory reset failed',
      });
    });
  });

  it('should handle restore system flow', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const restoreButton = screen.getByTestId('maintenance-restore-button');
    await user.click(restoreButton);

    // File input should be created
    await waitFor(() => {
      expect(document.createElement).toHaveBeenCalledWith('input');
    });
  });
});
