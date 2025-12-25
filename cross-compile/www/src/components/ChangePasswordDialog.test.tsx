/**
 * ChangePasswordDialog Component Tests
 */
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { LoginResult } from '@/services/authService';
import type { OnvifUser } from '@/services/userService';

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
}));

// Mock toast
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

// Mock UI components
vi.mock('@/components/ui/dialog', () => ({
  Dialog: ({
    children,
    open,
    onOpenChange,
  }: {
    children: React.ReactNode;
    open: boolean;
    onOpenChange: (open: boolean) => void;
  }) => (
    <div data-testid="dialog" data-open={open}>
      {open && (
        <div>
          <button onClick={() => onOpenChange(false)} data-testid="dialog-overlay">
            Close
          </button>
          {children}
        </div>
      )}
    </div>
  ),
  DialogContent: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div data-testid="dialog-content" className={className}>
      {children}
    </div>
  ),
  DialogDescription: ({
    children,
    ...props
  }: { children: React.ReactNode } & Record<string, unknown>) => (
    <div data-testid="dialog-description" {...props}>
      {children}
    </div>
  ),
  DialogFooter: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="dialog-footer">{children}</div>
  ),
  DialogHeader: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="dialog-header">{children}</div>
  ),
  DialogTitle: ({
    children,
    ...props
  }: { children: React.ReactNode } & Record<string, unknown>) => (
    <h2 data-testid="dialog-title" {...props}>
      {children}
    </h2>
  ),
}));

vi.mock('@/components/ui/button', () => ({
  Button: ({
    children,
    onClick,
    disabled,
    type,
    variant,
    className,
    ...props
  }: {
    children: React.ReactNode;
    onClick?: () => void;
    disabled?: boolean;
    type?: 'submit' | 'reset' | 'button';
    variant?: string;
    className?: string;
    [key: string]: unknown;
  }) => (
    <button
      onClick={onClick}
      disabled={disabled}
      type={type}
      data-variant={variant}
      className={className}
      {...props}
    >
      {children}
    </button>
  ),
}));

// Use real Form component - it's needed for react-hook-form to work
vi.mock('@/components/ui/form', async () => {
  return await vi.importActual('@/components/ui/form');
});

// Use real Input component - it should work with react-hook-form
vi.mock('@/components/ui/input', async () => {
  return await vi.importActual('@/components/ui/input');
});

describe('ChangePasswordDialog', () => {
  const mockUser: OnvifUser = {
    username: 'admin',
    userLevel: 'Administrator' as const,
  };

  beforeEach(async () => {
    vi.clearAllMocks();
    const { useAuth } = await import('@/hooks/useAuth');
    vi.mocked(useAuth).mockReturnValue({
      username: 'admin',
      login: vi.fn(),
      isAuthenticated: true,
      logout: vi.fn(),
      getCredentials: vi.fn(),
      getBasicAuthHeader: vi.fn(),
    });
    const { getUsers } = await import('@/services/userService');
    vi.mocked(getUsers).mockResolvedValue([mockUser]);
    const { setUser } = await import('@/services/userService');
    vi.mocked(setUser).mockResolvedValue(undefined);
  });

  describe('Dialog Open/Close', () => {
    it('should render dialog when open is true', () => {
      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);
      expect(screen.getByTestId('dialog')).toHaveAttribute('data-open', 'true');
    });

    it('should not render dialog content when open is false', () => {
      render(<ChangePasswordDialog open={false} onOpenChange={vi.fn()} />);
      expect(screen.getByTestId('dialog')).toHaveAttribute('data-open', 'false');
    });

    it('should call onOpenChange when cancel button is clicked', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();

      render(<ChangePasswordDialog open={true} onOpenChange={onOpenChange} />);

      const cancelButton = screen.getByTestId('change-password-dialog-cancel-button');
      await user.click(cancelButton);

      expect(onOpenChange).toHaveBeenCalledWith(false);
    });

    it('should not close dialog when loading', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();
      const { verifyCredentials } = await import('@/services/authService');
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

      await user.type(currentPasswordInput, 'oldpass');
      await user.type(newPasswordInput, 'newpass123');
      await user.type(confirmPasswordInput, 'newpass123');

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
      });

      // Find and submit the form directly to trigger loading
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      form?.dispatchEvent(submitEvent);

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
      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const submitButton = screen.getByTestId('change-password-dialog-submit-button');
      await user.click(submitButton);

      // Form validation should prevent submission
      const { verifyCredentials } = await import('@/services/authService');
      await waitFor(() => {
        expect(verifyCredentials).not.toHaveBeenCalled();
      });
    });

    it('should show error for password too short', async () => {
      const user = userEvent.setup();
      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');

      await user.type(currentPasswordInput, 'oldpass');
      await user.type(newPasswordInput, '12345'); // Too short

      const submitButton = screen.getByTestId('change-password-dialog-submit-button');
      await user.click(submitButton);

      const { verifyCredentials } = await import('@/services/authService');
      await waitFor(() => {
        expect(verifyCredentials).not.toHaveBeenCalled();
      });
    });

    it('should show error for password too long', async () => {
      const user = userEvent.setup();
      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');

      await user.type(currentPasswordInput, 'oldpass');
      await user.type(newPasswordInput, 'a'.repeat(33)); // Too long

      const submitButton = screen.getByTestId('change-password-dialog-submit-button');
      await user.click(submitButton);

      const { verifyCredentials } = await import('@/services/authService');
      await waitFor(() => {
        expect(verifyCredentials).not.toHaveBeenCalled();
      });
    });

    it('should show error when passwords do not match', async () => {
      const user = userEvent.setup();
      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await user.type(currentPasswordInput, 'oldpass');
      await user.type(newPasswordInput, 'newpass123');
      await user.type(confirmPasswordInput, 'different');

      const submitButton = screen.getByTestId('change-password-dialog-submit-button');
      await user.click(submitButton);

      const { verifyCredentials } = await import('@/services/authService');
      await waitFor(() => {
        expect(verifyCredentials).not.toHaveBeenCalled();
      });
    });
  });

  describe('Password Visibility Toggle', () => {
    it('should toggle current password visibility', async () => {
      const user = userEvent.setup();
      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );

      // Initially should be password type
      expect(currentPasswordInput).toHaveAttribute('type', 'password');

      // Find toggle button (Eye icon)
      const currentPasswordToggle = screen.getByTestId('change-password-dialog-current-toggle');
      await user.click(currentPasswordToggle);
      await waitFor(() => {
        expect(currentPasswordInput).toHaveAttribute('type', 'text');
      });
    });

    it('should toggle new password visibility', async () => {
      const user = userEvent.setup();
      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');

      // Initially should be password type
      expect(newPasswordInput).toHaveAttribute('type', 'password');

      // Find toggle button
      const newPasswordToggle = screen.getByTestId('change-password-dialog-new-toggle');
      await user.click(newPasswordToggle);
      await waitFor(() => {
        expect(newPasswordInput).toHaveAttribute('type', 'text');
      });
    });
  });

  describe('Successful Password Change', () => {
    it('should successfully change password with valid inputs', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();
      const { verifyCredentials } = await import('@/services/authService');
      const { getUsers } = await import('@/services/userService');
      const { setUser } = await import('@/services/userService');
      const { useAuth } = await import('@/hooks/useAuth');
      const { toast } = await import('sonner');
      vi.mocked(verifyCredentials).mockResolvedValue({ success: true });

      render(<ChangePasswordDialog open={true} onOpenChange={onOpenChange} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await user.type(currentPasswordInput, 'oldpass');
      await user.type(newPasswordInput, 'newpass123');
      await user.type(confirmPasswordInput, 'newpass123');

      // Wait a bit for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
        expect(newPasswordInput).toHaveValue('newpass123');
        expect(confirmPasswordInput).toHaveValue('newpass123');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      form?.dispatchEvent(submitEvent);

      await waitFor(
        () => {
          expect(verifyCredentials).toHaveBeenCalled();
          expect(getUsers).toHaveBeenCalled();
          expect(setUser).toHaveBeenCalledWith('admin', 'newpass123', 'Administrator');
          const authHook = vi.mocked(useAuth)();
          expect(authHook.login).toHaveBeenCalledWith('admin', 'newpass123');
          expect(toast.success).toHaveBeenCalledWith('Password updated successfully');
          expect(onOpenChange).toHaveBeenCalledWith(false);
        },
        { timeout: 3000 },
      );
    });

    it('should reset form after successful password change', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();
      const { verifyCredentials } = await import('@/services/authService');
      vi.mocked(verifyCredentials).mockResolvedValue({ success: true });

      render(<ChangePasswordDialog open={true} onOpenChange={onOpenChange} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await user.type(currentPasswordInput, 'oldpass');
      await user.type(newPasswordInput, 'newpass123');
      await user.type(confirmPasswordInput, 'newpass123');

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
        expect(newPasswordInput).toHaveValue('newpass123');
        expect(confirmPasswordInput).toHaveValue('newpass123');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      form?.dispatchEvent(submitEvent);

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
      const { verifyCredentials } = await import('@/services/authService');
      const { getUsers } = await import('@/services/userService');
      const { setUser } = await import('@/services/userService');
      vi.mocked(verifyCredentials).mockResolvedValue({ success: false });

      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await user.type(currentPasswordInput, 'wrongpass');
      await user.type(newPasswordInput, 'newpass123');
      await user.type(confirmPasswordInput, 'newpass123');

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('wrongpass');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      form?.dispatchEvent(submitEvent);

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
      const { verifyCredentials } = await import('@/services/authService');
      const { getUsers } = await import('@/services/userService');
      const { setUser } = await import('@/services/userService');
      const { toast } = await import('sonner');
      vi.mocked(verifyCredentials).mockResolvedValue({ success: true });
      vi.mocked(getUsers).mockResolvedValue([]); // No users

      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await user.type(currentPasswordInput, 'oldpass');
      await user.type(newPasswordInput, 'newpass123');
      await user.type(confirmPasswordInput, 'newpass123');

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      form?.dispatchEvent(submitEvent);

      await waitFor(
        () => {
          expect(toast.error).toHaveBeenCalledWith('Could not find user information');
          expect(setUser).not.toHaveBeenCalled();
        },
        { timeout: 3000 },
      );
    });

    it('should show error toast on network failure', async () => {
      const user = userEvent.setup();
      const error = new Error('Network error');
      const { verifyCredentials } = await import('@/services/authService');
      const { toast } = await import('sonner');
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

      await user.type(currentPasswordInput, 'oldpass');
      await user.type(newPasswordInput, 'newpass123');
      await user.type(confirmPasswordInput, 'newpass123');

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      form?.dispatchEvent(submitEvent);

      await waitFor(
        () => {
          expect(toast.error).toHaveBeenCalledWith(
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
      let resolveVerify: (value: { success: boolean }) => void;
      const verifyPromise = new Promise<{ success: boolean }>((resolve) => {
        resolveVerify = resolve;
      });
      const { verifyCredentials } = await import('@/services/authService');
      vi.mocked(verifyCredentials).mockReturnValue(verifyPromise);

      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await user.type(currentPasswordInput, 'oldpass');
      await user.type(newPasswordInput, 'newpass123');
      await user.type(confirmPasswordInput, 'newpass123');

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      const submitButton = screen.getByTestId('change-password-dialog-submit-button');

      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      form?.dispatchEvent(submitEvent);

      await waitFor(
        () => {
          expect(screen.getByText('Updating...')).toBeInTheDocument();
          expect(submitButton).toBeDisabled();
        },
        { timeout: 3000 },
      );

      // Resolve the promise
      resolveVerify!({ success: true });
      await verifyPromise;
    });

    it('should disable form inputs during loading', async () => {
      const user = userEvent.setup();
      let resolveVerify: (value: { success: boolean }) => void;
      const verifyPromise = new Promise<{ success: boolean }>((resolve) => {
        resolveVerify = resolve;
      });
      const { verifyCredentials } = await import('@/services/authService');
      vi.mocked(verifyCredentials).mockReturnValue(verifyPromise);

      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);

      const currentPasswordInput = screen.getByTestId(
        'change-password-dialog-current-password-input',
      );
      const newPasswordInput = screen.getByTestId('change-password-dialog-new-password-input');
      const confirmPasswordInput = screen.getByTestId(
        'change-password-dialog-confirm-password-input',
      );

      await user.type(currentPasswordInput, 'oldpass');
      await user.type(newPasswordInput, 'newpass123');
      await user.type(confirmPasswordInput, 'newpass123');

      // Wait for form state to update
      await waitFor(() => {
        expect(currentPasswordInput).toHaveValue('oldpass');
      });

      // Find and submit the form directly
      const form = currentPasswordInput.closest('form');
      expect(form).toBeTruthy();

      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      form?.dispatchEvent(submitEvent);

      await waitFor(
        () => {
          expect(currentPasswordInput).toBeDisabled();
          expect(newPasswordInput).toBeDisabled();
          expect(confirmPasswordInput).toBeDisabled();
        },
        { timeout: 3000 },
      );

      resolveVerify!({ success: true });
      await verifyPromise;
    });
  });

  describe('Dialog Description', () => {
    it('should display username in description', async () => {
      const { useAuth } = await import('@/hooks/useAuth');
      vi.mocked(useAuth).mockReturnValue({
        username: 'testuser',
        login: vi.fn(),
        isAuthenticated: true,
        logout: vi.fn(),
        getCredentials: vi.fn(),
        getBasicAuthHeader: vi.fn(),
      });

      render(<ChangePasswordDialog open={true} onOpenChange={vi.fn()} />);
      expect(screen.getByTestId('change-password-dialog-description')).toHaveTextContent(
        `Update your account password for testuser`,
      );
    });
  });
});
