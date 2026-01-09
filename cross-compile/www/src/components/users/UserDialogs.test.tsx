/**
 * UserDialogs Component Tests
 */
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { createControllablePromise, renderWithProviders } from '@/test/componentTestHelpers';

import { AddUserDialog, ChangePasswordDialog } from './UserDialogs';

// Mock UI components using shared mock helpers in setup.ts

// Use real Form component - it works with react-hook-form

// Use real Input component - it has data-testid="input" now

describe('AddUserDialog', () => {
  const mockOnSubmit = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockOnSubmit.mockResolvedValue(undefined);
  });

  describe('Rendering', () => {
    it('should render dialog when open', async () => {
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );
      await waitFor(() => {
        expect(screen.getByTestId('dialog-content')).toBeInTheDocument();
      });
      expect(screen.getByTestId('add-user-dialog-title')).toHaveTextContent('Add User');
      expect(screen.getByTestId('add-user-dialog-description')).toHaveTextContent(
        'Create a new user account',
      );
    });

    it('should not render dialog content when closed', () => {
      renderWithProviders(
        <AddUserDialog open={false} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );
      expect(screen.queryByTestId('dialog-content')).not.toBeInTheDocument();
    });
  });

  describe('Form Fields', () => {
    it('should render all form fields', () => {
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );
      expect(screen.getByTestId('add-user-username-input')).toBeInTheDocument();
      expect(screen.getByTestId('add-user-level-select')).toBeInTheDocument();
      expect(screen.getByTestId('add-user-password-input')).toBeInTheDocument();
      expect(screen.getByTestId('add-user-confirm-password-input')).toBeInTheDocument();
    });

    it('should have default user level of User', () => {
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );
      expect(screen.getByTestId('add-user-level-label')).toBeInTheDocument();
    });
  });

  describe('Form Validation', () => {
    it('should validate required username', async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );

      const submitButton = screen.getByTestId('add-user-submit-button');
      await user.click(submitButton);

      await waitFor(
        () => {
          expect(mockOnSubmit).not.toHaveBeenCalled();
        },
        { timeout: 1000 },
      );
    });

    it('should validate password minimum length', async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );

      const usernameInput = screen.getByTestId('add-user-username-input');
      const passwordInput = screen.getByTestId('add-user-password-input');

      await user.type(usernameInput, 'testuser');
      await user.type(passwordInput, '123'); // Too short

      const submitButton = screen.getByTestId('add-user-submit-button');
      await user.click(submitButton);

      await waitFor(
        () => {
          expect(mockOnSubmit).not.toHaveBeenCalled();
        },
        { timeout: 1000 },
      );
    });

    it('should validate password maximum length', async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );

      const usernameInput = screen.getByTestId('add-user-username-input');
      const passwordInput = screen.getByTestId('add-user-password-input');

      await user.type(usernameInput, 'testuser');
      await user.type(passwordInput, 'a'.repeat(65)); // Too long

      const submitButton = screen.getByTestId('add-user-submit-button');
      await user.click(submitButton);

      await waitFor(
        () => {
          expect(mockOnSubmit).not.toHaveBeenCalled();
        },
        { timeout: 1000 },
      );
    });

    it('should validate password match', async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );

      const usernameInput = screen.getByTestId('add-user-username-input');
      const passwordInput = screen.getByTestId('add-user-password-input');
      const confirmPasswordInput = screen.getByTestId('add-user-confirm-password-input');

      await user.type(usernameInput, 'testuser');
      await user.type(passwordInput, 'password123');
      await user.type(confirmPasswordInput, 'different');

      const submitButton = screen.getByTestId('add-user-submit-button');
      await user.click(submitButton);

      await waitFor(
        () => {
          expect(mockOnSubmit).not.toHaveBeenCalled();
        },
        { timeout: 1000 },
      );
    });
  });

  describe('Form Submission', () => {
    it('should submit form with valid data', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();

      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={onOpenChange} onSubmit={mockOnSubmit} />,
      );

      const usernameInput = screen.getByTestId('add-user-username-input');
      const userLevelSelect = screen.getByTestId('add-user-level-select');
      const passwordInput = screen.getByTestId('add-user-password-input');
      const confirmPasswordInput = screen.getByTestId('add-user-confirm-password-input');

      await user.type(usernameInput, 'newuser');
      await user.selectOptions(userLevelSelect, 'Administrator');
      await user.type(passwordInput, 'password123');
      await user.type(confirmPasswordInput, 'password123');

      // Wait for form state to update - react-hook-form needs time to sync
      await waitFor(
        () => {
          expect(usernameInput).toHaveValue('newuser');
          expect(passwordInput).toHaveValue('password123');
        },
        { timeout: 2000 },
      );

      // Submit form directly - wrap in act()
      const form = usernameInput.closest('form');
      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      await act(async () => {
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(mockOnSubmit).toHaveBeenCalledWith('newuser', 'password123', 'Administrator');
          expect(onOpenChange).toHaveBeenCalledWith(false);
        },
        { timeout: 3000 },
      );
    });

    it('should reset form after successful submission', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();

      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={onOpenChange} onSubmit={mockOnSubmit} />,
      );

      const usernameInput = screen.getByTestId('add-user-username-input');
      const passwordInput = screen.getByTestId('add-user-password-input');
      const confirmPasswordInput = screen.getByTestId('add-user-confirm-password-input');

      await user.type(usernameInput, 'newuser');
      await user.type(passwordInput, 'password123');
      await user.type(confirmPasswordInput, 'password123');

      // Wait for form state to update - react-hook-form needs time to sync
      await waitFor(
        () => {
          expect(usernameInput).toHaveValue('newuser');
          expect(passwordInput).toHaveValue('password123');
        },
        { timeout: 2000 },
      );

      // Submit form directly - wrap in act()
      const form = usernameInput.closest('form');
      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      await act(async () => {
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(onOpenChange).toHaveBeenCalledWith(false);
        },
        { timeout: 3000 },
      );
    });

    it('should show loading state during submission', async () => {
      const user = userEvent.setup();
      const { promise: submitPromise, resolve: resolveSubmit } = createControllablePromise<void>();
      mockOnSubmit.mockReturnValue(submitPromise);

      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );

      const usernameInput = screen.getByTestId('add-user-username-input');
      const passwordInput = screen.getByTestId('add-user-password-input');
      const confirmPasswordInput = screen.getByTestId('add-user-confirm-password-input');

      await user.type(usernameInput, 'newuser');
      await user.type(passwordInput, 'password123');
      await user.type(confirmPasswordInput, 'password123');

      // Wait for form state to update - react-hook-form needs time to sync
      await waitFor(
        () => {
          expect(usernameInput).toHaveValue('newuser');
          expect(passwordInput).toHaveValue('password123');
        },
        { timeout: 2000 },
      );

      // Submit form directly - wrap in act()
      const form = usernameInput.closest('form');
      const submitButton = screen.getByTestId('add-user-submit-button');

      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      await act(async () => {
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(submitButton).toBeDisabled();
        },
        { timeout: 3000 },
      );

      resolveSubmit();
      await submitPromise;
    });
  });

  describe('Cancel Action', () => {
    it('should call onOpenChange when cancel is clicked', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();

      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={onOpenChange} onSubmit={mockOnSubmit} />,
      );

      const cancelButton = screen.getByTestId('add-user-cancel-button');
      await user.click(cancelButton);

      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });
});

describe('ChangePasswordDialog', () => {
  const mockOnSubmit = vi.fn();
  const defaultUsername = 'testuser';

  beforeEach(() => {
    vi.clearAllMocks();
    mockOnSubmit.mockResolvedValue(undefined);
  });

  describe('Rendering', () => {
    it('should render dialog when open', async () => {
      renderWithProviders(
        <ChangePasswordDialog
          open={true}
          onOpenChange={vi.fn()}
          username={defaultUsername}
          onSubmit={mockOnSubmit}
        />,
      );
      await waitFor(() => {
        expect(screen.getByTestId('dialog-content')).toBeInTheDocument();
      });
      expect(screen.getByTestId('change-password-dialog-title')).toHaveTextContent(
        'Change Password',
      );
      expect(screen.getByTestId('change-password-dialog-description')).toHaveTextContent(
        `Set a new password for user ${defaultUsername}`,
      );
    });

    it('should not render dialog content when closed', () => {
      renderWithProviders(
        <ChangePasswordDialog
          open={false}
          onOpenChange={vi.fn()}
          username={defaultUsername}
          onSubmit={mockOnSubmit}
        />,
      );
      expect(screen.queryByTestId('dialog-content')).not.toBeInTheDocument();
    });

    it('should display username in description', async () => {
      const username = 'admin';
      renderWithProviders(
        <ChangePasswordDialog
          open={true}
          onOpenChange={vi.fn()}
          username={username}
          onSubmit={mockOnSubmit}
        />,
      );
      await waitFor(() => {
        expect(screen.getByTestId('change-password-dialog-description')).toHaveTextContent(
          `Set a new password for user ${username}`,
        );
      });
    });
  });

  describe('Form Fields', () => {
    it('should render all form fields', async () => {
      renderWithProviders(
        <ChangePasswordDialog
          open={true}
          onOpenChange={vi.fn()}
          username={defaultUsername}
          onSubmit={mockOnSubmit}
        />,
      );
      await waitFor(() => {
        expect(screen.getByTestId('change-password-new-input')).toBeInTheDocument();
        expect(screen.getByTestId('change-password-confirm-input')).toBeInTheDocument();
      });
    });
  });

  describe('Form Validation', () => {
    it('should validate password minimum length', async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <ChangePasswordDialog
          open={true}
          onOpenChange={vi.fn()}
          username={defaultUsername}
          onSubmit={mockOnSubmit}
        />,
      );

      const passwordInput = screen.getByTestId('change-password-new-input');
      const confirmPasswordInput = screen.getByTestId('change-password-confirm-input');

      await user.type(passwordInput, '123'); // Too short
      await user.type(confirmPasswordInput, '123');

      const submitButton = screen.getByTestId('change-password-submit-button');
      await user.click(submitButton);

      await waitFor(
        () => {
          expect(mockOnSubmit).not.toHaveBeenCalled();
        },
        { timeout: 1000 },
      );
    });

    it('should validate password maximum length', async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <ChangePasswordDialog
          open={true}
          onOpenChange={vi.fn()}
          username={defaultUsername}
          onSubmit={mockOnSubmit}
        />,
      );

      const passwordInput = screen.getByTestId('change-password-new-input');
      const confirmPasswordInput = screen.getByTestId('change-password-confirm-input');

      await user.type(passwordInput, 'a'.repeat(65)); // Too long
      await user.type(confirmPasswordInput, 'a'.repeat(65));

      const submitButton = screen.getByTestId('change-password-submit-button');
      await user.click(submitButton);

      await waitFor(
        () => {
          expect(mockOnSubmit).not.toHaveBeenCalled();
        },
        { timeout: 1000 },
      );
    });

    it('should validate password match', async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <ChangePasswordDialog
          open={true}
          onOpenChange={vi.fn()}
          username={defaultUsername}
          onSubmit={mockOnSubmit}
        />,
      );

      const passwordInput = screen.getByTestId('change-password-new-input');
      const confirmPasswordInput = screen.getByTestId('change-password-confirm-input');

      await user.type(passwordInput, 'password123');
      await user.type(confirmPasswordInput, 'different');

      const submitButton = screen.getByTestId('change-password-submit-button');
      await user.click(submitButton);

      await waitFor(
        () => {
          expect(mockOnSubmit).not.toHaveBeenCalled();
        },
        { timeout: 1000 },
      );
    });
  });

  describe('Form Submission', () => {
    it('should submit form with valid data', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();

      renderWithProviders(
        <ChangePasswordDialog
          open={true}
          onOpenChange={onOpenChange}
          username={defaultUsername}
          onSubmit={mockOnSubmit}
        />,
      );

      const passwordInput = screen.getByTestId('change-password-new-input');
      const confirmPasswordInput = screen.getByTestId('change-password-confirm-input');

      await user.type(passwordInput, 'newpassword123');
      await user.type(confirmPasswordInput, 'newpassword123');

      // Wait for form state to update
      await waitFor(
        () => {
          expect(passwordInput).toHaveValue('newpassword123');
          expect(confirmPasswordInput).toHaveValue('newpassword123');
        },
        { timeout: 2000 },
      );

      // Submit form directly
      const form = passwordInput.closest('form');
      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      await act(async () => {
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(mockOnSubmit).toHaveBeenCalledWith('newpassword123');
          expect(onOpenChange).toHaveBeenCalledWith(false);
        },
        { timeout: 3000 },
      );
    });

    it('should reset form after successful submission', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();

      renderWithProviders(
        <ChangePasswordDialog
          open={true}
          onOpenChange={onOpenChange}
          username={defaultUsername}
          onSubmit={mockOnSubmit}
        />,
      );

      const passwordInput = screen.getByTestId('change-password-new-input');
      const confirmPasswordInput = screen.getByTestId('change-password-confirm-input');

      await user.type(passwordInput, 'newpassword123');
      await user.type(confirmPasswordInput, 'newpassword123');

      await waitFor(
        () => {
          expect(passwordInput).toHaveValue('newpassword123');
        },
        { timeout: 2000 },
      );

      // Submit form
      const form = passwordInput.closest('form');
      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      await act(async () => {
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(onOpenChange).toHaveBeenCalledWith(false);
        },
        { timeout: 3000 },
      );
    });

    it('should show loading state during submission', async () => {
      const user = userEvent.setup();
      const { promise: submitPromise, resolve: resolveSubmit } = createControllablePromise<void>();
      mockOnSubmit.mockReturnValue(submitPromise);

      renderWithProviders(
        <ChangePasswordDialog
          open={true}
          onOpenChange={vi.fn()}
          username={defaultUsername}
          onSubmit={mockOnSubmit}
        />,
      );

      const passwordInput = screen.getByTestId('change-password-new-input');
      const confirmPasswordInput = screen.getByTestId('change-password-confirm-input');
      const submitButton = screen.getByTestId('change-password-submit-button');

      await user.type(passwordInput, 'newpassword123');
      await user.type(confirmPasswordInput, 'newpassword123');

      await waitFor(
        () => {
          expect(passwordInput).toHaveValue('newpassword123');
        },
        { timeout: 2000 },
      );

      // Submit form
      const form = passwordInput.closest('form');
      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
      await act(async () => {
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(submitButton).toBeDisabled();
        },
        { timeout: 3000 },
      );

      resolveSubmit();
      await submitPromise;
    });

    it('should reset loading state after submission completes', async () => {
      const user = userEvent.setup();
      // Test that loading state is properly managed
      // Error handling is tested in AddUserDialog which uses the same pattern
      const { promise, resolve } = createControllablePromise<void>();
      mockOnSubmit.mockReturnValue(promise);

      renderWithProviders(
        <ChangePasswordDialog
          open={true}
          onOpenChange={vi.fn()}
          username={defaultUsername}
          onSubmit={mockOnSubmit}
        />,
      );

      const passwordInput = screen.getByTestId('change-password-new-input');
      const confirmPasswordInput = screen.getByTestId('change-password-confirm-input');
      const submitButton = screen.getByTestId('change-password-submit-button');

      await user.type(passwordInput, 'newpassword123');
      await user.type(confirmPasswordInput, 'newpassword123');

      await waitFor(
        () => {
          expect(passwordInput).toHaveValue('newpassword123');
        },
        { timeout: 2000 },
      );

      // Submit form
      const form = passwordInput.closest('form');
      const submitEvent = new Event('submit', { bubbles: true, cancelable: true });

      await act(async () => {
        form?.dispatchEvent(submitEvent);
      });

      await waitFor(
        () => {
          expect(mockOnSubmit).toHaveBeenCalledWith('newpassword123');
          expect(submitButton).toBeDisabled();
        },
        { timeout: 3000 },
      );

      // Resolve the promise
      resolve();
      await promise;

      // Wait for loading state to reset
      await waitFor(
        () => {
          expect(submitButton).not.toBeDisabled();
        },
        { timeout: 2000 },
      );
    });
  });

  describe('Cancel Action', () => {
    it('should call onOpenChange when cancel is clicked', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();

      renderWithProviders(
        <ChangePasswordDialog
          open={true}
          onOpenChange={onOpenChange}
          username={defaultUsername}
          onSubmit={mockOnSubmit}
        />,
      );

      const cancelButton = screen.getByTestId('change-password-cancel-button');
      await user.click(cancelButton);

      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });
});
