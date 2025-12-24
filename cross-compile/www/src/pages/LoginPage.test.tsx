/**
 * LoginPage Tests
 */
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { AuthProvider } from '@/hooks/useAuth';

import LoginPage from './LoginPage';

// Mock authService
vi.mock('@/services/authService', () => ({
  verifyCredentials: vi.fn(),
}));

// Mock react-router-dom
const mockNavigate = vi.fn();
const mockLocation = { state: null, pathname: '/login', hash: '', search: '', key: 'test' };

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
    useLocation: () => mockLocation,
  };
});

// Mock sonner toast
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

// Helper function to create delayed promise for testing loading states
const createDelayedPromise = (delay = 100): Promise<{ success: boolean }> => {
  return new Promise((resolve) => {
    setTimeout(() => resolve({ success: true }), delay);
  });
};

describe('LoginPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset sessionStorage
    sessionStorage.clear();
  });

  it('should render login form', async () => {
    render(
      <AuthProvider>
        <LoginPage />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByText('ONVIF Device Manager')).toBeInTheDocument();
    });
    expect(screen.getByPlaceholderText(/enter username/i)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/enter password/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument();
  });

  it('should show validation error for empty username', async () => {
    const user = userEvent.setup();
    render(
      <AuthProvider>
        <LoginPage />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/enter username/i)).toBeInTheDocument();
    });

    const submitButton = screen.getByRole('button', { name: /sign in/i });
    await user.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText(/username is required/i)).toBeInTheDocument();
    });
  });

  it('should show validation error for empty password', async () => {
    const user = userEvent.setup();
    render(
      <AuthProvider>
        <LoginPage />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/enter username/i)).toBeInTheDocument();
    });

    const usernameInput = screen.getByPlaceholderText(/enter username/i);
    await user.type(usernameInput, 'testuser');

    const submitButton = screen.getByRole('button', { name: /sign in/i });
    await user.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText(/password is required/i)).toBeInTheDocument();
    });
  });

  it('should toggle password visibility', async () => {
    const user = userEvent.setup();
    render(
      <AuthProvider>
        <LoginPage />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/enter password/i)).toBeInTheDocument();
    });

    const passwordInput = screen.getByPlaceholderText(/enter password/i);
    await user.type(passwordInput, 'testpassword');

    // Password should be hidden by default
    expect(passwordInput).toHaveAttribute('type', 'password');

    // Find and click the toggle button (it's a sibling in the relative container)
    const passwordContainer = passwordInput.closest('.relative');
    const toggleButton = passwordContainer?.querySelector('button[type="button"]');
    expect(toggleButton).toBeInTheDocument();

    if (toggleButton) {
      await user.click(toggleButton);
      await waitFor(() => {
        expect(passwordInput).toHaveAttribute('type', 'text');
      });

      await user.click(toggleButton);
      await waitFor(() => {
        expect(passwordInput).toHaveAttribute('type', 'password');
      });
    }
  });

  it('should submit form with valid credentials', async () => {
    const { verifyCredentials } = await import('@/services/authService');
    vi.mocked(verifyCredentials).mockResolvedValue({ success: true });

    const user = userEvent.setup();
    render(
      <AuthProvider>
        <LoginPage />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/enter username/i)).toBeInTheDocument();
    });

    const usernameInput = screen.getByPlaceholderText(/enter username/i);
    const passwordInput = screen.getByPlaceholderText(/enter password/i);
    const submitButton = screen.getByRole('button', { name: /sign in/i });

    await user.type(usernameInput, 'testuser');
    await user.type(passwordInput, 'testpassword');
    await user.click(submitButton);

    await waitFor(() => {
      expect(verifyCredentials).toHaveBeenCalledWith('testuser', 'testpassword');
    });
  });

  it('should show loading state during submission', async () => {
    const { verifyCredentials } = await import('@/services/authService');
    vi.mocked(verifyCredentials).mockImplementation(() => createDelayedPromise(100));

    const user = userEvent.setup();
    render(
      <AuthProvider>
        <LoginPage />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/enter username/i)).toBeInTheDocument();
    });

    const usernameInput = screen.getByPlaceholderText(/enter username/i);
    const passwordInput = screen.getByPlaceholderText(/enter password/i);
    const submitButton = screen.getByRole('button', { name: /sign in/i });

    await user.type(usernameInput, 'testuser');
    await user.type(passwordInput, 'testpassword');
    await user.click(submitButton);

    // Should show loading state
    await waitFor(() => {
      expect(screen.getByText(/signing in/i)).toBeInTheDocument();
    });
    expect(submitButton).toBeDisabled();
  });
});
