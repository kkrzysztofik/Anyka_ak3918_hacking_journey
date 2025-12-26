/**
 * LoginPage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  createDelayedPromise,
  // This import should remain as createDelayedPromise is now exported from componentTestHelpers
  mockToast,
  renderWithProviders,
} from '@/test/componentTestHelpers';

import LoginPage from './LoginPage';

// Mock authService
vi.mock('@/services/authService', () => ({
  verifyCredentials: vi.fn(),
}));

// Mock react-router-dom
const mockNavigate = vi.fn();
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const mockLocation = { state: null as any, pathname: '/login', hash: '', search: '', key: 'test' };

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
    useLocation: () => mockLocation,
  };
});

describe('LoginPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset sessionStorage
    sessionStorage.clear();
  });

  it('should render login form', async () => {
    renderWithProviders(<LoginPage />);

    await waitFor(() => {
      expect(screen.getByTestId('login-page-title')).toHaveTextContent('ONVIF Device Manager');
    });
    expect(screen.getByTestId('login-form-username-input')).toBeInTheDocument();
    expect(screen.getByTestId('login-form-password-input')).toBeInTheDocument();
    expect(screen.getByTestId('login-form-submit-button')).toBeInTheDocument();
  });

  it('should show validation error for empty username', async () => {
    const user = userEvent.setup();
    renderWithProviders(<LoginPage />);

    await waitFor(() => {
      expect(screen.getByTestId('login-form-username-input')).toBeInTheDocument();
    });

    const submitButton = screen.getByTestId('login-form-submit-button');
    await user.click(submitButton);

    await waitFor(() => {
      expect(screen.getByTestId('login-username-error')).toHaveTextContent(/username is required/i);
    });
  });

  it('should show validation error for empty password', async () => {
    const user = userEvent.setup();
    renderWithProviders(<LoginPage />);

    await waitFor(() => {
      expect(screen.getByTestId('login-form-username-input')).toBeInTheDocument();
    });

    const usernameInput = screen.getByTestId('login-form-username-input');
    await user.type(usernameInput, 'testuser');

    const submitButton = screen.getByTestId('login-form-submit-button');
    await user.click(submitButton);

    await waitFor(() => {
      expect(screen.getByTestId('login-password-error')).toHaveTextContent(/password is required/i);
    });
  });

  it('should toggle password visibility', async () => {
    const user = userEvent.setup();
    renderWithProviders(<LoginPage />);

    await waitFor(() => {
      expect(screen.getByTestId('login-form-password-input')).toBeInTheDocument();
    });

    const passwordInput = screen.getByTestId('login-form-password-input');
    await user.type(passwordInput, 'testpassword');

    // Password should be hidden by default
    expect(passwordInput).toHaveAttribute('type', 'password');

    // Find and click the toggle button
    const toggleButton = screen.getByTestId('login-form-password-toggle-button');
    expect(toggleButton).toBeInTheDocument();

    await user.click(toggleButton);
    await waitFor(() => {
      expect(passwordInput).toHaveAttribute('type', 'text');
    });

    await user.click(toggleButton);
    await waitFor(() => {
      expect(passwordInput).toHaveAttribute('type', 'password');
    });
  });

  it('should submit form with valid credentials', async () => {
    const { verifyCredentials } = await import('@/services/authService');
    vi.mocked(verifyCredentials).mockResolvedValue({ success: true });

    const user = userEvent.setup();
    renderWithProviders(<LoginPage />);

    await waitFor(() => {
      expect(screen.getByTestId('login-form-username-input')).toBeInTheDocument();
    });

    const usernameInput = screen.getByTestId('login-form-username-input');
    const passwordInput = screen.getByTestId('login-form-password-input');
    const submitButton = screen.getByTestId('login-form-submit-button');

    await user.type(usernameInput, 'testuser');
    await user.type(passwordInput, 'testpassword');
    await user.click(submitButton);

    await waitFor(() => {
      expect(verifyCredentials).toHaveBeenCalledWith('testuser', 'testpassword');
    });
  });

  it('should show loading state during submission', async () => {
    const { verifyCredentials } = await import('@/services/authService');
    vi.mocked(verifyCredentials).mockImplementation(() =>
      createDelayedPromise({ success: true }, 100),
    );

    const user = userEvent.setup();
    renderWithProviders(<LoginPage />);

    await waitFor(() => {
      expect(screen.getByTestId('login-form-username-input')).toBeInTheDocument();
    });

    const usernameInput = screen.getByTestId('login-form-username-input');
    const passwordInput = screen.getByTestId('login-form-password-input');
    const submitButton = screen.getByTestId('login-form-submit-button');

    await user.type(usernameInput, 'testuser');
    await user.type(passwordInput, 'testpassword');
    await user.click(submitButton);

    // Should show loading state
    await waitFor(() => {
      expect(screen.getByTestId('login-loading-text')).toHaveTextContent(/signing in/i);
    });
    expect(submitButton).toBeDisabled();
  });

  it('should redirect when already authenticated', async () => {
    // Set up authenticated state
    sessionStorage.setItem(
      'onvif_camera_auth',
      JSON.stringify({
        username: 'admin',
        encryptedPassword: { iv: 'test', data: 'test', tag: 'test' },
      }),
    );

    renderWithProviders(<LoginPage />);

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/', { replace: true });
    });
  });

  it('should redirect to safe path from location state', async () => {
    // Set up authenticated state
    sessionStorage.setItem(
      'onvif_camera_auth',
      JSON.stringify({
        username: 'admin',
        encryptedPassword: { iv: 'test', data: 'test', tag: 'test' },
      }),
    );

    // Mock location with safe path
    mockLocation.state = { from: { pathname: '/settings' } };

    renderWithProviders(<LoginPage />);

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/settings', { replace: true });
    });
  });

  it('should not redirect to unsafe path (double slash)', async () => {
    // Set up authenticated state
    sessionStorage.setItem(
      'onvif_camera_auth',
      JSON.stringify({
        username: 'admin',
        encryptedPassword: { iv: 'test', data: 'test', tag: 'test' },
      }),
    );

    // Mock location with unsafe path
    mockLocation.state = { from: { pathname: '//evil.com' } };

    renderWithProviders(<LoginPage />);

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/', { replace: true });
    });
  });

  it('should show error toast when authentication fails', async () => {
    const { verifyCredentials } = await import('@/services/authService');
    vi.mocked(verifyCredentials).mockResolvedValue({
      success: false,
      error: 'Invalid credentials',
    });

    const user = userEvent.setup();
    renderWithProviders(<LoginPage />);

    await waitFor(() => {
      expect(screen.getByTestId('login-form-username-input')).toBeInTheDocument();
    });

    const usernameInput = screen.getByTestId('login-form-username-input');
    const passwordInput = screen.getByTestId('login-form-password-input');
    const submitButton = screen.getByTestId('login-form-submit-button');

    await user.type(usernameInput, 'testuser');
    await user.type(passwordInput, 'wrongpassword');
    await user.click(submitButton);

    await waitFor(() => {
      expect(mockToast.error).toHaveBeenCalledWith('Invalid credentials');
    });
  });

  it('should handle login error', async () => {
    const { verifyCredentials } = await import('@/services/authService');
    vi.mocked(verifyCredentials).mockRejectedValue(new Error('Invalid credentials'));

    const user = userEvent.setup();
    renderWithProviders(<LoginPage />);

    const usernameInput = screen.getByTestId('login-form-username-input');
    const passwordInput = screen.getByTestId('login-form-password-input');
    const submitButton = screen.getByTestId('login-form-submit-button');

    await user.type(usernameInput, 'admin');
    await user.type(passwordInput, 'wrongpassword');
    await user.click(submitButton);

    await waitFor(() => {
      expect(mockToast.error).toHaveBeenCalledWith('An error occurred during sign in');
    });
  });
});
