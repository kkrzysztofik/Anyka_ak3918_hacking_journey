/**
 * Layout Component Tests
 */
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import Layout from './Layout';

// Mock useAuth hook
const mockLogout = vi.fn();
const mockUseAuth = vi.fn();

vi.mock('@/hooks/useAuth', () => ({
  useAuth: () => mockUseAuth(),
}));

// Mock child components
vi.mock('@/components/AboutDialog', () => ({
  AboutDialog: ({
    open,
    onOpenChange,
  }: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
  }) => (
    <div data-testid="about-dialog">
      {open ? (
        <div>
          <button onClick={() => onOpenChange(false)}>Close About</button>
        </div>
      ) : null}
    </div>
  ),
}));

vi.mock('@/components/ChangePasswordDialog', () => ({
  ChangePasswordDialog: ({
    open,
    onOpenChange,
  }: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
  }) => (
    <div data-testid="change-password-dialog">
      {open ? (
        <div>
          <button onClick={() => onOpenChange(false)}>Close Change Password</button>
        </div>
      ) : null}
    </div>
  ),
}));

vi.mock('@/components/common/ConnectionStatus', () => ({
  ConnectionStatusBadge: ({ status }: { status: string }) => (
    <div data-testid="connection-status">{status}</div>
  ),
}));

// Mock UI components
vi.mock('@/components/ui/sheet', () => ({
  Sheet: ({
    children,
    open,
    onOpenChange: _onOpenChange,
  }: {
    children: React.ReactNode;
    open: boolean;
    onOpenChange: (open: boolean) => void;
  }) => (
    <div data-testid="sheet" data-open={open}>
      {children}
    </div>
  ),
  SheetContent: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="sheet-content">{children}</div>
  ),
  SheetTrigger: ({
    children,
    asChild: _asChild,
  }: {
    children: React.ReactNode;
    asChild?: boolean;
  }) => <div data-testid="sheet-trigger">{children}</div>,
}));

vi.mock('@/components/ui/button', () => ({
  Button: ({
    children,
    onClick,
    ...props
  }: {
    children: React.ReactNode;
    onClick?: () => void;
    [key: string]: unknown;
  }) => (
    <button onClick={onClick} {...props}>
      {children}
    </button>
  ),
}));

describe('Layout', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseAuth.mockReturnValue({
      username: 'admin',
      logout: mockLogout,
      isAuthenticated: true,
    });
  });

  const renderLayout = (initialPath = '/') => {
    return render(
      <MemoryRouter initialEntries={[initialPath]}>
        <Layout />
      </MemoryRouter>,
    );
  };

  describe('Rendering', () => {
    it('should render with authenticated user', () => {
      renderLayout();
      const sidebar = screen.getByTestId('layout-desktop-sidebar');
      expect(within(sidebar).getByTestId('layout-branding')).toBeInTheDocument();
      expect(within(sidebar).getByTestId('layout-branding')).toHaveTextContent('ONVIF');
      expect(within(sidebar).getByTestId('layout-branding')).toHaveTextContent('Device Manager');
      // User is rendered in the sidebar footer button which we can check too, but 'admin' text is checked by getAllByText originally.
      // We can check layout-user-menu-button which I think is where the user name is.
      expect(within(sidebar).getByTestId('layout-user-menu-button')).toHaveTextContent('admin');
    });

    it('should render navigation items', () => {
      renderLayout();
      expect(screen.getAllByText('Live View')[0]).toBeInTheDocument();
      expect(screen.getAllByText('Settings')[0]).toBeInTheDocument();
      expect(screen.getAllByText('Diagnostics')[0]).toBeInTheDocument();
    });

    it('should render connection status badge', () => {
      renderLayout();
      expect(screen.getByTestId('connection-status')).toBeInTheDocument();
      expect(screen.getByTestId('connection-status')).toHaveTextContent('connected');
    });

    it('should render mobile header on small screens', () => {
      renderLayout();
      // Header should be visible (it's always rendered, just hidden on large screens)
      expect(screen.getByTestId('layout-header')).toBeInTheDocument();
    });
  });

  describe('Navigation', () => {
    it('should highlight active navigation item', () => {
      renderLayout('/live');
      const liveViewLinks = screen.getAllByText('Live View');
      const liveViewLink = liveViewLinks[0].closest('a');
      expect(liveViewLink).toHaveAttribute('href', '/live');
    });

    it('should expand settings submenu when settings path is active', async () => {
      renderLayout('/settings/network');
      // Settings should be expanded to show children
      await waitFor(() => {
        const networkElements = screen.getAllByText('Network');
        expect(networkElements.length).toBeGreaterThan(0);
      });
    });

    it('should toggle settings submenu on click', async () => {
      const user = userEvent.setup();
      renderLayout();

      const settingsButtons = screen.getAllByTestId('layout-nav-settings-toggle-button');
      const settingsButton = settingsButtons[0]; // Use desktop version
      expect(settingsButton).toBeInTheDocument();

      // Initially, submenu might be collapsed or expanded based on route
      // Click to toggle
      await user.click(settingsButton);
      // After click, submenu should be visible
      await waitFor(() => {
        expect(screen.getByText('Identification')).toBeInTheDocument();
      });
    });

    it('should render all settings submenu items', async () => {
      const user = userEvent.setup();
      renderLayout();

      const settingsButtons = screen.getAllByTestId('layout-nav-settings-toggle-button');
      const settingsButton = settingsButtons[0]; // Use desktop version
      await user.click(settingsButton);
      await waitFor(() => {
        expect(screen.getByText('Identification')).toBeInTheDocument();
        const networkElements = screen.getAllByText('Network');
        expect(networkElements.length).toBeGreaterThan(0);
        expect(screen.getByText('Time')).toBeInTheDocument();
        expect(screen.getByText('Imaging')).toBeInTheDocument();
        expect(screen.getByText('Profiles')).toBeInTheDocument();
        expect(screen.getByText('Users')).toBeInTheDocument();
        expect(screen.getByText('Maintenance')).toBeInTheDocument();
      });
    });
  });

  describe('User Menu', () => {
    it('should toggle user menu on click', async () => {
      const user = userEvent.setup();
      renderLayout();

      const userButtons = screen.getAllByTestId('layout-user-menu-button');
      const userButton = userButtons[0]; // Use desktop version
      expect(userButton).toBeInTheDocument();

      await user.click(userButton);
      await waitFor(() => {
        expect(screen.getByText('Change Password')).toBeInTheDocument();
        expect(screen.getByText('About')).toBeInTheDocument();
        expect(screen.getByText('Sign Out')).toBeInTheDocument();
      });
    });

    it('should close user menu when clicking outside', async () => {
      const user = userEvent.setup();
      renderLayout();

      const userButtons = screen.getAllByTestId('layout-user-menu-button');
      const userButton = userButtons[0]; // Use desktop version
      await user.click(userButton);
      await waitFor(() => {
        expect(screen.getByText('Change Password')).toBeInTheDocument();
      });

      // Click outside button (the backdrop)
      const backdrop = screen.getByLabelText('Close menu');
      await user.click(backdrop);

      await waitFor(() => {
        expect(screen.queryByText('Change Password')).not.toBeInTheDocument();
      });
    });

    it('should open change password dialog', async () => {
      const user = userEvent.setup();
      renderLayout();

      const userButtons = screen.getAllByTestId('layout-user-menu-button');
      const userButton = userButtons[0]; // Use desktop version
      await user.click(userButton);
      await waitFor(() => {
        expect(screen.getByText('Change Password')).toBeInTheDocument();
      });

      const changePasswordButton = screen.getByTestId('layout-change-password-button');
      await user.click(changePasswordButton);

      await waitFor(() => {
        const dialogs = screen.getAllByTestId('change-password-dialog');
        expect(dialogs.length).toBeGreaterThan(0);
      });
    });

    it('should open about dialog', async () => {
      const user = userEvent.setup();
      renderLayout();

      const userButtons = screen.getAllByTestId('layout-user-menu-button');
      const userButton = userButtons[0]; // Use desktop version
      await user.click(userButton);
      await waitFor(() => {
        expect(screen.getByText('About')).toBeInTheDocument();
      });

      const aboutButton = screen.getByTestId('layout-about-button');
      await user.click(aboutButton);

      await waitFor(() => {
        const dialogs = screen.getAllByTestId('about-dialog');
        expect(dialogs.length).toBeGreaterThan(0);
      });
    });

    it('should call logout when sign out is clicked', async () => {
      const user = userEvent.setup();
      renderLayout();

      const userButtons = screen.getAllByTestId('layout-user-menu-button');
      const userButton = userButtons[0]; // Use desktop version
      await user.click(userButton);
      await waitFor(() => {
        expect(screen.getByText('Sign Out')).toBeInTheDocument();
      });

      const signOutButton = screen.getByTestId('layout-sign-out-button');
      await user.click(signOutButton);

      expect(mockLogout).toHaveBeenCalledTimes(1);
    });
  });

  describe('Mobile Navigation', () => {
    it('should render mobile sheet menu trigger', () => {
      renderLayout();
      expect(screen.getByTestId('sheet-trigger')).toBeInTheDocument();
    });

    it('should render sheet content with sidebar', () => {
      renderLayout();
      // Sheet content should be rendered (even if not visible)
      // The sheet might be closed initially, so we just check it exists in the DOM
      expect(screen.getByTestId('sheet')).toBeInTheDocument();
    });
  });

  describe('Page Title', () => {
    it('should display correct page title for live view', () => {
      renderLayout('/live');
      // The header should show the page title
      expect(screen.getByTestId('layout-page-title')).toHaveTextContent('Live View');
      const header = screen.getByTestId('layout-header');
      expect(header).toBeInTheDocument();
    });

    it('should display correct page title for settings', () => {
      renderLayout('/settings/network');
      expect(screen.getByTestId('layout-page-title')).toHaveTextContent('Network');
      const header = screen.getByTestId('layout-header');
      expect(header).toBeInTheDocument();
    });
  });

  describe('Outlet Rendering', () => {
    it('should render outlet for child routes', () => {
      renderLayout();
      // Outlet is rendered by react-router, we just verify the main structure exists
      const main = screen.getByRole('main');
      expect(main).toBeInTheDocument();
    });
  });

  describe('User Display', () => {
    it('should display username from auth context', () => {
      mockUseAuth.mockReturnValue({
        username: 'testuser',
        logout: mockLogout,
        isAuthenticated: true,
      });

      renderLayout();
      const testuserElements = screen.getAllByText('testuser');
      expect(testuserElements.length).toBeGreaterThan(0);
    });

    it('should display Administrator role', () => {
      renderLayout();
      const adminElements = screen.getAllByText('Administrator');
      expect(adminElements.length).toBeGreaterThan(0);
    });

    it('should display version information', () => {
      renderLayout();
      const versionElements = screen.getAllByText(/v1.0.0-beta/);
      expect(versionElements.length).toBeGreaterThan(0);
      const onvifTexts = screen.getAllByText(/ONVIF 24.12/);
      expect(onvifTexts.length).toBeGreaterThan(0);
    });
  });
});
