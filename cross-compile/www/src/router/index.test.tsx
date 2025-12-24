/**
 * Router Tests
 */
import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { AuthProvider } from '@/hooks/useAuth';

import AppRouter, { LoadingFallback } from './index';

// Mock the pages to avoid loading them
vi.mock('@/pages/LoginPage', () => ({
  default: () => <div>Login Page</div>,
}));

vi.mock('@/pages/LiveViewPage', () => ({
  default: () => <div>Live View Page</div>,
}));

vi.mock('@/pages/DiagnosticsPage', () => ({
  default: () => <div>Diagnostics Page</div>,
}));

vi.mock('@/pages/settings/IdentificationPage', () => ({
  default: () => <div>Identification Page</div>,
}));

vi.mock('@/pages/settings/NetworkPage', () => ({
  default: () => <div>Network Page</div>,
}));

vi.mock('@/pages/settings/TimePage', () => ({
  default: () => <div>Time Page</div>,
}));

vi.mock('@/pages/settings/ImagingPage', () => ({
  default: () => <div>Imaging Page</div>,
}));

vi.mock('@/pages/settings/UserManagementPage', () => ({
  default: () => <div>User Management Page</div>,
}));

vi.mock('@/pages/settings/MaintenancePage', () => ({
  default: () => <div>Maintenance Page</div>,
}));

vi.mock('@/pages/settings/ProfilesPage', () => ({
  default: () => <div>Profiles Page</div>,
}));

vi.mock('@/Layout', () => ({
  default: ({ children }: { children: React.ReactNode }) => <div>Layout{children}</div>,
}));

describe('Router', () => {
  describe('LoadingFallback', () => {
    it('should render loading spinner', () => {
      const { container } = render(<LoadingFallback />);
      const spinner = container.querySelector('.animate-spin');
      expect(spinner).toBeInTheDocument();
    });
  });

  describe('ProtectedRoute', () => {
    it('should redirect to login when not authenticated', async () => {
      globalThis.history.replaceState(null, '', '#/live');
      render(
        <AuthProvider>
          <AppRouter />
        </AuthProvider>,
      );

      // Should redirect to login when not authenticated
      await waitFor(
        () => {
          expect(screen.getByText('Login Page')).toBeInTheDocument();
        },
        { timeout: 3000 },
      );
    });
  });

  describe('AppRouter', () => {
    beforeEach(() => {
      // Reset hash before each test
      globalThis.location.hash = '';
    });

    it('should render login page at /login', async () => {
      // Use history API to set hash
      globalThis.history.replaceState(null, '', '#/login');
      render(
        <AuthProvider>
          <AppRouter />
        </AuthProvider>,
      );

      await waitFor(
        () => {
          expect(screen.getByText('Login Page')).toBeInTheDocument();
        },
        { timeout: 3000 },
      );
    });

    it('should redirect to login when accessing protected route without auth', async () => {
      globalThis.history.replaceState(null, '', '#/live');
      render(
        <AuthProvider>
          <AppRouter />
        </AuthProvider>,
      );

      await waitFor(
        () => {
          expect(screen.getByText('Login Page')).toBeInTheDocument();
        },
        { timeout: 3000 },
      );
    });
  });
});
