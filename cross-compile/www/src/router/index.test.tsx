import { render, screen, waitFor } from '@testing-library/react';
import { Outlet } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { AuthProvider } from '@/hooks/useAuth';

import AppRouter, { LoadingFallback } from './index';

// Mock the pages to avoid loading them
vi.mock('@/pages/LoginPage', () => ({
  default: () => <div data-testid="page-login">Login Page</div>,
}));

vi.mock('@/pages/LiveViewPage', () => ({
  default: () => <div data-testid="page-live">Live View Page</div>,
}));

vi.mock('@/pages/DiagnosticsPage', () => ({
  default: () => <div data-testid="page-diagnostics">Diagnostics Page</div>,
}));

vi.mock('@/pages/settings/IdentificationPage', () => ({
  default: () => <div data-testid="page-identification">Identification Page</div>,
}));

vi.mock('@/pages/settings/NetworkPage', () => ({
  default: () => <div data-testid="page-network">Network Page</div>,
}));

vi.mock('@/pages/settings/TimePage', () => ({
  default: () => <div data-testid="page-time">Time Page</div>,
}));

vi.mock('@/pages/settings/ImagingPage', () => ({
  default: () => <div data-testid="page-imaging">Imaging Page</div>,
}));

vi.mock('@/pages/settings/UserManagementPage', () => ({
  default: () => <div data-testid="page-users">User Management Page</div>,
}));

vi.mock('@/pages/settings/MaintenancePage', () => ({
  default: () => <div data-testid="page-maintenance">Maintenance Page</div>,
}));

vi.mock('@/pages/settings/ProfilesPage', () => ({
  default: () => <div data-testid="page-profiles">Profiles Page</div>,
}));

vi.mock('@/Layout', () => ({
  default: () => (
    <div data-testid="layout">
      Layout <Outlet />
    </div>
  ),
}));

describe('Router', () => {
  describe('LoadingFallback', () => {
    it('should render loading spinner', () => {
      const { container } = render(<LoadingFallback />);
      const spinner = container.querySelector('.animate-spin');
      expect(spinner).toBeInTheDocument();
    });
  });

  describe('AppRouter', () => {
    beforeEach(() => {
      globalThis.location.hash = '';
      sessionStorage.clear();
    });

    const setAuth = () => {
      sessionStorage.setItem(
        'onvif_camera_auth',
        JSON.stringify({
          username: 'admin',
          encryptedPassword: { iv: 'test', data: 'test', tag: 'test' },
        }),
      );
    };

    it('should render login page at /login', async () => {
      globalThis.history.replaceState(null, '', '#/login');
      render(
        <AuthProvider>
          <AppRouter />
        </AuthProvider>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('page-login')).toBeInTheDocument();
      });
    });

    it('should redirect to login when accessing protected route without auth', async () => {
      globalThis.history.replaceState(null, '', '#/live');
      render(
        <AuthProvider>
          <AppRouter />
        </AuthProvider>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('page-login')).toBeInTheDocument();
      });
    });

    // Authenticated routes tests
    const routes = [
      { path: '#/live', testId: 'page-live' },
      { path: '#/diagnostics', testId: 'page-diagnostics' },
      { path: '#/settings/identification', testId: 'page-identification' },
      { path: '#/settings/network', testId: 'page-network' },
      { path: '#/settings/time', testId: 'page-time' },
      { path: '#/settings/imaging', testId: 'page-imaging' },
      { path: '#/settings/users', testId: 'page-users' },
      { path: '#/settings/maintenance', testId: 'page-maintenance' },
      { path: '#/settings/profiles', testId: 'page-profiles' },
      { path: '#/', testId: 'page-live' }, // Redirects to /live
      { path: '#/settings', testId: 'page-identification' }, // Redirects to /settings/identification
      { path: '#/unknown/route', testId: 'page-live' }, // Catch-all redirects to /live
    ];

    it.each(routes)('should render correct page for $path', async ({ path, testId }) => {
      setAuth();
      globalThis.history.replaceState(null, '', path);
      render(
        <AuthProvider>
          <AppRouter />
        </AuthProvider>,
      );

      await waitFor(
        () => {
          expect(screen.getByTestId('layout')).toBeInTheDocument();
          expect(screen.getByTestId(testId)).toBeInTheDocument();
        },
        { timeout: 5000 },
      );
    });

    it('should preserve navigation state when redirecting to login', async () => {
      globalThis.history.replaceState(null, '', '#/live');
      render(
        <AuthProvider>
          <AppRouter />
        </AuthProvider>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('page-login')).toBeInTheDocument();
      });
    });
  });
});
