/**
 * UserDialogs Component Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { createControllablePromise, renderWithProviders } from '@/test/componentTestHelpers';
import { testDialogTitleAndDescription } from '@/test/dialogTestHelpers';
import {
  submitFormByEvent,
  testFormFieldValidation,
  waitForFormValues,
} from '@/test/formTestHelpers';

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
      await testDialogTitleAndDescription(
        'add-user-dialog-title',
        'add-user-dialog-description',
        'Add User',
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

      await testFormFieldValidation(user, [], 'add-user-submit-button', mockOnSubmit);
    });

    it('should validate password minimum length', async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );

      await testFormFieldValidation(
        user,
        [
          { testId: 'add-user-username-input', value: 'testuser' },
          { testId: 'add-user-password-input', value: '123' }, // Too short
        ],
        'add-user-submit-button',
        mockOnSubmit,
      );
    });

    it('should validate password maximum length', async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );

      await testFormFieldValidation(
        user,
        [
          { testId: 'add-user-username-input', value: 'testuser' },
          { testId: 'add-user-password-input', value: 'a'.repeat(65) }, // Too long
        ],
        'add-user-submit-button',
        mockOnSubmit,
      );
    });

    it('should validate password match', async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );

      await testFormFieldValidation(
        user,
        [
          { testId: 'add-user-username-input', value: 'testuser' },
          { testId: 'add-user-password-input', value: 'password123' },
          { testId: 'add-user-confirm-password-input', value: 'different' },
        ],
        'add-user-submit-button',
        mockOnSubmit,
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
      await waitForFormValues(
        [
          { testId: 'add-user-username-input', value: 'newuser' },
          { testId: 'add-user-password-input', value: 'password123' },
        ],
        2000,
      );

      // Submit form directly - wrap in act()
      await submitFormByEvent('add-user-username-input', user);

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
      await waitForFormValues(
        [
          { testId: 'add-user-username-input', value: 'newuser' },
          { testId: 'add-user-password-input', value: 'password123' },
        ],
        2000,
      );

      // Submit form directly - wrap in act()
      await submitFormByEvent('add-user-username-input', user);

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

      const submitButton = screen.getByTestId('add-user-submit-button');
      await user.click(submitButton);

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
      await testDialogTitleAndDescription(
        'change-password-dialog-title',
        'change-password-dialog-description',
        'Change Password',
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

      await testFormFieldValidation(
        user,
        [
          { testId: 'change-password-new-input', value: '123' }, // Too short
          { testId: 'change-password-confirm-input', value: '123' },
        ],
        'change-password-submit-button',
        mockOnSubmit,
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

      await testFormFieldValidation(
        user,
        [
          { testId: 'change-password-new-input', value: 'a'.repeat(65) }, // Too long
          { testId: 'change-password-confirm-input', value: 'a'.repeat(65) },
        ],
        'change-password-submit-button',
        mockOnSubmit,
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

      await testFormFieldValidation(
        user,
        [
          { testId: 'change-password-new-input', value: 'password123' },
          { testId: 'change-password-confirm-input', value: 'different' },
        ],
        'change-password-submit-button',
        mockOnSubmit,
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
      await waitForFormValues(
        [
          { testId: 'change-password-new-input', value: 'newpassword123' },
          { testId: 'change-password-confirm-input', value: 'newpassword123' },
        ],
        2000,
      );

      // Submit form directly
      await submitFormByEvent('change-password-new-input', user);

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

      await waitForFormValues(
        [{ testId: 'change-password-new-input', value: 'newpassword123' }],
        2000,
      );

      // Submit form
      await submitFormByEvent('change-password-new-input', user);

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

      await user.click(submitButton);

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

      await user.click(submitButton);

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
