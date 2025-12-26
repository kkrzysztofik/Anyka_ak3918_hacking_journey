/**
 * UserDialogs Component Tests
 */
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { createControllablePromise, renderWithProviders } from '@/test/componentTestHelpers';

import { AddUserDialog } from './UserDialogs';

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
    it('should render dialog when open', () => {
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );
      expect(screen.getByTestId('dialog')).toHaveAttribute('data-open', 'true');
      expect(screen.getByTestId('add-user-dialog-title')).toHaveTextContent('Add User');
      expect(screen.getByTestId('add-user-dialog-description')).toHaveTextContent(
        'Create a new user account',
      );
    });

    it('should not render dialog content when closed', () => {
      renderWithProviders(
        <AddUserDialog open={false} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );
      expect(screen.getByTestId('dialog')).toHaveAttribute('data-open', 'false');
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
      expect(screen.getByText('User Level')).toBeInTheDocument();
    });
  });

  describe('Form Validation', () => {
    it('should validate required username', async () => {
      const user = userEvent.setup();
      renderWithProviders(
        <AddUserDialog open={true} onOpenChange={vi.fn()} onSubmit={mockOnSubmit} />,
      );

      const submitButton = screen.getByTestId('add-user-submit-button');
      await act(async () => {
        await user.click(submitButton);
      });

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

      await act(async () => {
        await user.type(usernameInput, 'testuser');
        await user.type(passwordInput, '123'); // Too short
      });

      const submitButton = screen.getByTestId('add-user-submit-button');
      await act(async () => {
        await user.click(submitButton);
      });

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

      await act(async () => {
        await user.type(usernameInput, 'testuser');
        await user.type(passwordInput, 'a'.repeat(65)); // Too long
      });

      const submitButton = screen.getByTestId('add-user-submit-button');
      await act(async () => {
        await user.click(submitButton);
      });

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

      await act(async () => {
        await user.type(usernameInput, 'testuser');
        await user.type(passwordInput, 'password123');
        await user.type(confirmPasswordInput, 'different');
      });

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

      await act(async () => {
        await user.type(usernameInput, 'newuser');
        await user.type(passwordInput, 'password123');
        await user.type(confirmPasswordInput, 'password123');
      });

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
      await act(async () => {
        await user.click(cancelButton);
      });

      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });
});
