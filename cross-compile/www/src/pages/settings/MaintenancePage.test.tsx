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
import {
  closeDialog,
  mockToast,
  openDialog,
  renderWithProviders,
} from '@/test/componentTestHelpers';
import {
  testMutationWithErrorToast,
  testMutationWithSuccessToastAndDescription,
} from '@/test/mutationTestHelpers';

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
    expect(screen.getByTestId('maintenance-title')).toBeInTheDocument();
    expect(screen.getByTestId('maintenance-backup-restore-title')).toBeInTheDocument();
    expect(screen.getByTestId('maintenance-firmware-title')).toBeInTheDocument();
    expect(screen.getByTestId('maintenance-soft-reset-title')).toBeInTheDocument();
    expect(screen.getByTestId('maintenance-hard-reset-title')).toBeInTheDocument();
    expect(screen.getByTestId('maintenance-reboot-title')).toBeInTheDocument();
  });

  it('should open and close reboot dialog', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    await openDialog(user, 'maintenance-reboot-button', 'maintenance-reboot-dialog-title');
    await closeDialog(user, 'maintenance-reboot-cancel-button', 'maintenance-reboot-dialog-title');
  });

  it('should call reboot mutation on confirm', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const rebootButton = screen.getByTestId('maintenance-reboot-button');
    await user.click(rebootButton);

    await waitFor(() => {
      expect(screen.getByTestId('maintenance-reboot-dialog-title')).toBeInTheDocument();
    });

    await testMutationWithSuccessToastAndDescription(
      user,
      'maintenance-reboot-confirm-button',
      systemReboot,
      'Device is rebooting',
      'Please wait for the device to restart...',
    );
  });

  it('should show error toast when reboot fails', async () => {
    vi.mocked(systemReboot).mockRejectedValue(new Error('Network error'));

    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const rebootButton = screen.getByTestId('maintenance-reboot-button');
    await user.click(rebootButton);

    await waitFor(() => {
      expect(screen.getByTestId('maintenance-reboot-dialog-title')).toBeInTheDocument();
    });

    await testMutationWithErrorToast(
      user,
      'maintenance-reboot-confirm-button',
      systemReboot,
      'Failed to reboot device',
      'Network error',
    );
  });

  it('should open and close soft reset dialog', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    await openDialog(user, 'maintenance-soft-reset-button', 'maintenance-soft-reset-dialog-title');
    await closeDialog(
      user,
      'maintenance-soft-reset-cancel-button',
      'maintenance-soft-reset-dialog-title',
    );
  });

  it('should call soft reset mutation on confirm', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const softResetButton = screen.getByTestId('maintenance-soft-reset-button');
    await user.click(softResetButton);

    await waitFor(() => {
      expect(screen.getByTestId('maintenance-soft-reset-dialog-title')).toBeInTheDocument();
    });

    await testMutationWithSuccessToastAndDescription(
      user,
      'maintenance-soft-reset-confirm-button',
      setSystemFactoryDefault,
      'Device is resetting',
      'Settings returned to defaults. Device will reboot.',
      ['Soft'],
    );
  });

  it('should open and close hard reset dialog', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    await openDialog(user, 'maintenance-hard-reset-button', 'maintenance-hard-reset-dialog-title');
    await closeDialog(
      user,
      'maintenance-hard-reset-cancel-button',
      'maintenance-hard-reset-dialog-title',
    );
  });

  it('should call hard reset mutation on confirm', async () => {
    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const hardResetButton = screen.getByTestId('maintenance-hard-reset-button');
    await user.click(hardResetButton);

    await waitFor(() => {
      // Dialog should be open
      expect(screen.getByTestId('maintenance-hard-reset-dialog-title')).toBeInTheDocument();
    });

    await testMutationWithSuccessToastAndDescription(
      user,
      'maintenance-hard-reset-confirm-button',
      setSystemFactoryDefault,
      'Factory reset initiated',
      'All data will be erased. Device will reboot.',
      ['Hard'],
    );
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
      expect(screen.getByTestId('maintenance-soft-reset-dialog-title')).toBeInTheDocument();
    });

    await testMutationWithErrorToast(
      user,
      'maintenance-soft-reset-confirm-button',
      setSystemFactoryDefault,
      'Failed to reset settings',
      'Reset failed',
    );
  });

  it('should show error when hard reset fails', async () => {
    vi.mocked(setSystemFactoryDefault).mockRejectedValue(new Error('Factory reset failed'));

    const user = userEvent.setup();
    renderWithProviders(<MaintenancePage />);

    const hardResetButton = screen.getByTestId('maintenance-hard-reset-button');
    await user.click(hardResetButton);

    await waitFor(() => {
      expect(screen.getByTestId('maintenance-hard-reset-dialog-title')).toBeInTheDocument();
    });

    await testMutationWithErrorToast(
      user,
      'maintenance-hard-reset-confirm-button',
      setSystemFactoryDefault,
      'Failed to factory reset',
      'Factory reset failed',
    );
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
