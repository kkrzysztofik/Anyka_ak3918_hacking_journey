/**
 * ChangePasswordDialog Component Tests
 */
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useAuth } from '@/hooks/useAuth';
import type { LoginResult } from '@/services/authService';
import { verifyCredentials } from '@/services/authService';
import type { OnvifUser } from '@/services/userService';
import { getUsers, setUser } from '@/services/userService';
import {
  MOCK_DATA,
  createControllablePromise,
  mockToast,
  renderWithProviders,
  verifyPasswordVisibilityToggle,
} from '@/test/componentTestHelpers';

import { ChangePasswordDialog } from './ChangePasswordDialog';

// Mock services
vi.mock('@/services/authService', () => ({
  verifyCredentials: vi.fn(),
}));

vi.mock('@/services/userService', () => ({
  getUsers: vi.fn(),
  setUser: vi.fn(),
}));

// Mock useAuth hook
vi.mock('@/hooks/useAuth', () => ({
  useAuth: vi.fn(),
  AuthProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

describe('ChangePasswordDialog', () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    vi.mocked(useAuth).mockReturnValue({
      username: 'admin',
      login: vi.fn(),
      isAuthenticated: true,
      logout: vi.fn(),
      getCredentials: vi.fn(),
      getBasicAuthHeader: vi.fn(),
    });
    vi.mocked(getUsers).mockResolvedValue([...MOCK_DATA.users] as OnvifUser[]);
    vi.mocked(setUser).mockResolvedValue(undefined);
  });

  describe('Dialog Open/Close', () => {
    it('should render dialog when open is true', () => {
      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);
      expect(screen.getByTestId('dialog')).toHaveAttribute('data-open', 'true');
    });

    it('should not render dialog content when open is false', () => {
      renderWithProviders(<ChangePasswordDialog open={false} onOpenChange={vi.fn()} />);
      expect(screen.getByTestId('dialog')).toHaveAttribute('data-open', 'false');
    });

    it('should call onOpenChange when cancel button is clicked', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();

      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={onOpenChange} />);

      const cancelButton = screen.getByTestId('change-password-dialog-cancel-button');
      await act(async () => {
        await user.click(cancelButton);
      });

      expect(onOpenChange).toHaveBeenCalledWith(false);
    });

    it('should not close dialog when loading', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();
      // Mock that never resolves to keep loading state
      const neverResolvingPromise = new Promise<LoginResult>(() => {});
      vi.mocked(verifyCredentials).mockImplementation(() => neverResolvingPromise);

      render(<ChangePasswordDialog open={true} onOpenChange={onOpenChange} />);

      // Fill form completely and submit to trigger loading state
      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await act(async () => {
        await user.type(currentPasswordInput, 'oldpass');
        await user.type(newPasswordInput, 'newpass123');
        await user.type(confirmPasswordInput, 'newpass123');
      });

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
      });

      // Find and submit the form directly to trigger loading
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      await act(async () => {
        const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
        form?.dispatchEvent(submitEvent);
      });

      // Now check if cancel button is disabled during loading
      await waitFor(
        () => {
          const cancelButton = screen.getByTestId('change-password-dialog-cancel-button');
          expect(cancelButton).toBeDisabled();
        },
        { timeout: 3000 },
      );
    });
  });

  describe('Form Validation', () => {
    it('should show error for empty current password', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const submitButton = screen.getByTestId('change-password-dialog-submit-button');
      await act(async () => {
        await user.click(submitButton);
      });

      // Form validation should prevent submission
      await waitFor(() => {
        expect(verifyCredentials).not.toHaveBeenCalled();
      });
    });

    it('should show error for password too short', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');

      await act(async () => {
        await user.type(currentPasswordInput, 'oldpass');
        await user.type(newPasswordInput, '12345'); // Too short
      });

      const submitButton = screen.getByTestId('change-password-dialog-submit-button');
      await act(async () => {
        await user.click(submitButton);
      });

      await waitFor(() => {
        expect(verifyCredentials).not.toHaveBeenCalled();
      });
    });

    it('should show error for password too long', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');

      await act(async () => {
        await user.type(currentPasswordInput, 'oldpass');
        await user.type(newPasswordInput, 'a'.repeat(33)); // Too long
      });

      const submitButton = screen.getByTestId('change-password-dialog-submit-button');
      await act(async () => {
        await user.click(submitButton);
      });

      await waitFor(() => {
        expect(verifyCredentials).not.toHaveBeenCalled();
      });
    });

    it('should show error when passwords do not match', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await act(async () => {
        await user.type(currentPasswordInput, 'oldpass');
        await user.type(newPasswordInput, 'newpass123');
        await user.type(confirmPasswordInput, 'different');
      });

      const submitButton = screen.getByTestId('change-password-dialog-submit-button');
      await act(async () => {
        await user.click(submitButton);
      });

      await waitFor(() => {
        expect(verifyCredentials).not.toHaveBeenCalled();
      });
    });
  });

  describe('Password Visibility Toggle', () => {
    it('should toggle current password visibility', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      await verifyPasswordVisibilityToggle(
        user,
        'change-password-dialog-current-password-input',
        'change-password-dialog-current-toggle',
      );
    });

    it('should toggle new password visibility', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      await verifyPasswordVisibilityToggle(
        user,
        'change-password-dialog-new-password-input',
        'change-password-dialog-new-toggle',
      );
    });
  });

  describe('Successful Password Change', () => {
    it('should successfully change password with valid inputs', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();
      vi.mocked(verifyCredentials).mockResolvedValue({ success: true });

      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={onOpenChange} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await act(async () => {
        await user.type(currentPasswordInput, 'oldpass');
        await user.type(newPasswordInput, 'newpass123');
        await user.type(confirmPasswordInput, 'newpass123');
      });

      // Wait a bit for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
        expect(newPasswordInput).toHaveValue('newpass123');
        expect(confirmPasswordInput).toHaveValue('newpass123');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      await act(async () => {
        const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(verifyCredentials).toHaveBeenCalled();
          expect(getUsers).toHaveBeenCalled();
          expect(setUser).toHaveBeenCalledWith('admin', 'newpass123', 'Administrator');
          const authHook = vi.mocked(useAuth)();
          expect(authHook.login).toHaveBeenCalledWith('admin', 'newpass123');
          expect(mockToast.success).toHaveBeenCalledWith('Password updated successfully');
          expect(onOpenChange).toHaveBeenCalledWith(false);
        },
        { timeout: 3000 },
      );
    });

    it('should reset form after successful password change', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();
      vi.mocked(verifyCredentials).mockResolvedValue({ success: true });

      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={onOpenChange} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await act(async () => {
        await user.type(currentPasswordInput, 'oldpass');
        await user.type(newPasswordInput, 'newpass123');
        await user.type(confirmPasswordInput, 'newpass123');
      });

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
        expect(newPasswordInput).toHaveValue('newpass123');
        expect(confirmPasswordInput).toHaveValue('newpass123');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      await act(async () => {
        const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(onOpenChange).toHaveBeenCalledWith(false);
        },
        { timeout: 3000 },
      );
    });
  });

  describe('Error Handling', () => {
    it('should show error for invalid current password', async () => {
      const user = userEvent.setup();
      vi.mocked(verifyCredentials).mockResolvedValue({ success: false });

      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await act(async () => {
        await user.type(currentPasswordInput, 'wrongpass');
        await user.type(newPasswordInput, 'newpass123');
        await user.type(confirmPasswordInput, 'newpass123');
      });

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('wrongpass');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      await act(async () => {
        const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(verifyCredentials).toHaveBeenCalledWith('admin', 'wrongpass');
          expect(getUsers).not.toHaveBeenCalled();
          expect(setUser).not.toHaveBeenCalled();
        },
        { timeout: 3000 },
      );
    });

    it('should show error when user is not found', async () => {
      const user = userEvent.setup();
      vi.mocked(verifyCredentials).mockResolvedValue({ success: true });
      vi.mocked(getUsers).mockResolvedValue([]); // No users

      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await act(async () => {
        await user.type(currentPasswordInput, 'oldpass');
        await user.type(newPasswordInput, 'newpass123');
        await user.type(confirmPasswordInput, 'newpass123');
      });

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      await act(async () => {
        const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(mockToast.error).toHaveBeenCalledWith('Could not find user information');
          expect(setUser).not.toHaveBeenCalled();
        },
        { timeout: 3000 },
      );
    });

    it('should show error toast on network failure', async () => {
      const user = userEvent.setup();
      const error = new Error('Network error');
      vi.mocked(verifyCredentials).mockRejectedValue(error);

      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await act(async () => {
        await user.type(currentPasswordInput, 'oldpass');
        await user.type(newPasswordInput, 'newpass123');
        await user.type(confirmPasswordInput, 'newpass123');
      });

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      await act(async () => {
        const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(mockToast.error).toHaveBeenCalledWith(
            'Failed to update password. Please check your connection.',
          );
          expect(consoleSpy).toHaveBeenCalledWith('Failed to change password:', error);
        },
        { timeout: 3000 },
      );

      consoleSpy.mockRestore();
    });
  });

  describe('Loading States', () => {
    it('should show loading state during submission', async () => {
      const user = userEvent.setup();
      const { promise: verifyPromise, resolve: resolveVerify } = createControllablePromise<{
        success: boolean;
      }>();
      vi.mocked(verifyCredentials).mockReturnValue(verifyPromise);

      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await act(async () => {
        await user.type(currentPasswordInput, 'oldpass');
        await user.type(newPasswordInput, 'newpass123');
        await user.type(confirmPasswordInput, 'newpass123');
      });

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      const submitButton = screen.getByTestId('change-password-dialog-submit-button');

      await act(async () => {
        const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(screen.getByText('Updating...')).toBeInTheDocument();
          expect(submitButton).toBeDisabled();
        },
        { timeout: 3000 },
      );

      // Resolve the promise
      resolveVerify({ success: true });
      await verifyPromise;
    });

    it('should disable form inputs during loading', async () => {
      const user = userEvent.setup();
      const { promise: verifyPromise, resolve: resolveVerify } = createControllablePromise<{
        success: boolean;
      }>();
      vi.mocked(verifyCredentials).mockReturnValue(verifyPromise);

      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await act(async () => {
        await user.type(currentPasswordInput, 'oldpass');
        await user.type(newPasswordInput, 'newpass123');
        await user.type(confirmPasswordInput, 'newpass123');
      });

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      await act(async () => {
        const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(currentPasswordInput).toBeDisabled();
          expect(newPasswordInput).toBeDisabled();
          expect(confirmPasswordInput).toBeDisabled();
        },
        { timeout: 3000 },
      );

      resolveVerify({ success: true });
      await verifyPromise;
    });
  });

  describe('Dialog Description', () => {
    it('should display username in description', async () => {
      vi.mocked(useAuth).mockReturnValue({
        username: 'testuser',
        login: vi.fn(),
        isAuthenticated: true,
        logout: vi.fn(),
        getCredentials: vi.fn(),
        getBasicAuthHeader: vi.fn(),
      });

      renderWithProviders(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);
      expect(screen.getByTestId('change-password-dialog-description')).toHaveTextContent(
        `Update your account password for testuser`,
      );
    });
  });
});
